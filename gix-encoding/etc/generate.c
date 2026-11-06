/* Generate numeric mapping tables by querying the host iconv implementation.
 * No implementation source or character-map files are copied. See README.md.
 * Compile and invoke through generate.sh, from this directory. */
#include <errno.h>
#include <iconv.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define INVALID UINT32_MAX
#define CHILD UINT32_C(0x80000000)
#define PAIR UINT32_C(0x40000000)
#define MAX_NODES 512

static uint32_t nodes[MAX_NODES][256], reverse_map[0x110000];
static uint32_t pairs[16][2], pair_count, node_count;
static iconv_t decoder, encoder;
static void fail(const char *message);

static int flush(iconv_t descriptor, unsigned char output[32]) {
    char *out = (char *)output;
    size_t left = 32;
    if (iconv(descriptor, NULL, NULL, &out, &left) == (size_t)-1) fail("flush iconv");
    return 32 - left;
}

static void fail(const char *message) {
    perror(message);
    exit(1);
}

static void put32(FILE *file, uint32_t value) {
    unsigned char bytes[] = {value, value >> 8, value >> 16, value >> 24};
    if (fwrite(bytes, 1, sizeof bytes, file) != sizeof bytes) fail("write table");
}

static void put16(FILE *file, uint16_t value) {
    unsigned char bytes[] = {value, value >> 8};
    if (fwrite(bytes, 1, sizeof bytes, file) != sizeof bytes) fail("write table");
}

static uint32_t get32(const unsigned char *bytes) {
    return bytes[0] | (uint32_t)bytes[1] << 8 | (uint32_t)bytes[2] << 16 | (uint32_t)bytes[3] << 24;
}

/* Return -1 for invalid, -2 for incomplete, otherwise the number of output bytes. */
static int convert(iconv_t descriptor, const unsigned char *input, size_t length, unsigned char output[32]) {
    char *in = (char *)input, *out = (char *)output;
    size_t in_left = length, out_left = 32;
    if (iconv(descriptor, NULL, NULL, NULL, NULL) == (size_t)-1) fail("reset iconv");
    size_t result = iconv(descriptor, &in, &in_left, &out, &out_left);
    if (result == (size_t)-1) {
        if (errno == EINVAL) return -2;
        if (errno == EILSEQ) return -1;
        fail("iconv");
    }
    if (in_left) fail("iconv did not consume input");
    return 32 - out_left;
}

static void build_node(uint32_t node, unsigned char prefix[3], unsigned int depth, unsigned int max_depth) {
    for (unsigned int byte = 0; byte < 256; byte++) {
        unsigned char output[32];
        prefix[depth] = byte;
        int length = convert(decoder, prefix, depth + 1, output);
        if (length == 0) length = flush(decoder, output);
        if (length == -2 && depth + 1 < max_depth) {
            if (node_count == MAX_NODES) fail("too many trie nodes");
            /* Reserve a dense second-byte plane. The decoder can compute its
             * address directly from the lead byte without a dependent lookup. */
            uint32_t child = depth == 0 ? byte + 1 : node_count++;
            nodes[node][byte] = CHILD | child;
            build_node(child, prefix, depth + 1, max_depth);
        } else if (length == 4) {
            nodes[node][byte] = get32(output);
        } else if (length == 8) {
            if (pair_count == 16) fail("too many scalar pairs");
            pairs[pair_count][0] = get32(output);
            pairs[pair_count][1] = get32(output + 4);
            nodes[node][byte] = PAIR | pair_count++;
        } else if (length >= 0) {
            fprintf(stderr, "unexpected decoded length %d at depth %u\n", length, depth);
            exit(1);
        }
    }
}

static uint32_t packed(const unsigned char *bytes, int length) {
    uint32_t value = 0;
    for (int index = 0; index < length; index++) value = (value << 8) | bytes[index];
    return value;
}

static void generate(const char *name, const char *filename, unsigned int depth) {
    decoder = iconv_open("UTF-32LE", name);
    encoder = iconv_open(name, "UTF-32LE");
    if (decoder == (iconv_t)-1 || encoder == (iconv_t)-1) fail(name);
    memset(nodes, 0xff, sizeof nodes);
    memset(reverse_map, 0xff, sizeof reverse_map);
    node_count = depth > 1 ? 257 : 1;
    pair_count = 0;
    unsigned char prefix[3] = {0};
    build_node(0, prefix, 0, depth);
    for (uint32_t scalar = 0; scalar < 0x110000; scalar++) {
        if (scalar >= 0xd800 && scalar <= 0xdfff) continue;
        unsigned char input[] = {scalar, scalar >> 8, scalar >> 16, scalar >> 24}, output[32];
        int length = convert(encoder, input, 4, output);
        /* HKSCS holds these two scalars to combine a following diacritic. Its
         * flush result provides the standalone representation for our finish(). */
        if (length == 0) {
            length = flush(encoder, output);
        }
        if (length == 0) reverse_map[scalar] = UINT32_MAX - 1;
        else if (length > 0 && length <= 3) reverse_map[scalar] = packed(output, length);
        else if (length >= 0 && !(length == 4 && !strcmp(name, "GB18030"))) {
            fprintf(stderr, "unexpected encoded length %d for %s U+%04X\n", length, name, scalar);
            exit(1);
        }
    }

    uint16_t pages[256];
    uint32_t page_count = 0, supplementary_count = 0;
    for (unsigned int page = 0; page < 256; page++) {
        pages[page] = UINT16_MAX;
        if (depth > 1) {
            pages[page] = page_count++;
            continue;
        }
        for (unsigned int slot = 0; slot < 256; slot++) {
            if (reverse_map[page * 256 + slot] != INVALID) {
                pages[page] = page_count++;
                break;
            }
        }
    }
    for (uint32_t scalar = 0x10000; scalar < 0x110000; scalar++)
        if (reverse_map[scalar] != INVALID) supplementary_count++;

    FILE *file = fopen(filename, "wb");
    if (!file) fail(filename);
    put32(file, node_count);
    put32(file, page_count);
    put32(file, supplementary_count);
    put32(file, pair_count);
    for (uint32_t node = 0; node < node_count; node++)
        for (unsigned int byte = 0; byte < 256; byte++) put32(file, nodes[node][byte]);
    for (unsigned int page = 0; page < 256; page++) put16(file, pages[page]);
    for (unsigned int page = 0; page < 256; page++)
        if (pages[page] != UINT16_MAX)
            for (unsigned int slot = 0; slot < 256; slot++) put32(file, reverse_map[page * 256 + slot]);
    for (uint32_t scalar = 0x10000; scalar < 0x110000; scalar++)
        if (reverse_map[scalar] != INVALID) {
            put32(file, scalar);
            put32(file, reverse_map[scalar]);
        }
    for (uint32_t index = 0; index < pair_count; index++) {
        put32(file, pairs[index][0]);
        put32(file, pairs[index][1]);
    }
    if (fclose(file)) fail("close table");
    if (iconv_close(decoder) || iconv_close(encoder)) fail("close iconv");
    fprintf(stderr, "%s: %u decoder pages, %u encoder pages, %u supplementary scalars, %u pairs\n",
            name, node_count, page_count, supplementary_count, pair_count);
}

struct range { uint32_t pointer, length, scalar; };

static int scalar_order(const void *left, const void *right) {
    const struct range *a = left, *b = right;
    return a->scalar < b->scalar ? -1 : a->scalar > b->scalar;
}

static void generate_gb18030_ranges(void) {
    FILE *file = fopen("../src/tables/gb18030-ranges.bin", "wb");
    if (!file) fail("GB18030 ranges");
    decoder = iconv_open("UTF-32LE", "GB18030");
    if (decoder == (iconv_t)-1) fail("GB18030");
    uint32_t start = INVALID, first_scalar = 0, previous_scalar = 0;
    struct range ranges[1024];
    size_t count = 0;
    for (uint32_t pointer = 0; pointer <= 126 * 10 * 126 * 10; pointer++) {
        uint32_t rest = pointer;
        unsigned char bytes[4], output[32];
        bytes[3] = rest % 10 + 0x30; rest /= 10;
        bytes[2] = rest % 126 + 0x81; rest /= 126;
        bytes[1] = rest % 10 + 0x30; rest /= 10;
        bytes[0] = rest + 0x81;
        int length = pointer < 126 * 10 * 126 * 10 ? convert(decoder, bytes, 4, output) : -1;
        uint32_t scalar = length == 4 ? get32(output) : INVALID;
        if (start != INVALID && (scalar == INVALID || scalar != previous_scalar + 1)) {
            put32(file, start);
            put32(file, pointer - start);
            put32(file, first_scalar);
            if (count == sizeof ranges / sizeof *ranges) fail("too many GB18030 ranges");
            ranges[count++] = (struct range){start, pointer - start, first_scalar};
            start = INVALID;
        }
        if (scalar != INVALID && start == INVALID) {
            start = pointer;
            first_scalar = scalar;
        }
        previous_scalar = scalar;
    }
    if (fclose(file) || iconv_close(decoder)) fail("close GB18030 ranges");
    qsort(ranges, count, sizeof *ranges, scalar_order);
    file = fopen("../src/tables/gb18030-encode-ranges.bin", "wb");
    if (!file) fail("GB18030 encode ranges");
    for (size_t index = 0; index < count; index++) {
        put32(file, ranges[index].scalar);
        put32(file, ranges[index].length);
        put32(file, ranges[index].pointer);
    }
    if (fclose(file)) fail("close GB18030 encode ranges");
}

/* These iconv decoders compose adjacent Hebrew/Vietnamese characters. Infer
 * their small finite state machines from observable input/output and flush
 * behavior; a state's identity is its one buffered Unicode scalar. */
static void generate_combining(const char *name) {
    struct state {
        uint32_t scalar;
        unsigned char prefix[8];
        unsigned int length;
        uint32_t transitions[256][3];
    } states[256];
    memset(states, 0, sizeof states);
    states[0].scalar = INVALID;
    unsigned int state_count = 1;
    decoder = iconv_open("UTF-32LE", name);
    if (decoder == (iconv_t)-1) fail(name);
    for (unsigned int state = 0; state < state_count; state++) {
        for (unsigned int byte = 0; byte < 256; byte++) {
            unsigned char input[9], output[32], pending[32];
            memcpy(input, states[state].prefix, states[state].length);
            input[states[state].length] = byte;
            unsigned int input_length = states[state].length + 1;
            int length = convert(decoder, input, input_length, output);
            uint32_t *transition = states[state].transitions[byte];
            transition[0] = transition[1] = INVALID;
            transition[2] = 0;
            if (length < 0) continue;
            if (length != 0 && length != 4 && length != 8) fail("invalid composition output");
            transition[0] = length > 0 ? get32(output) : INVALID - 1;
            transition[1] = length > 4 ? get32(output + 4) : INVALID - 1;
            int pending_length = flush(decoder, pending);
            if (pending_length == 0) continue;
            if (pending_length != 4) fail("composition buffered more than one scalar");
            uint32_t scalar = get32(pending);
            unsigned int next;
            for (next = 1; next < state_count; next++)
                if (states[next].scalar == scalar) break;
            if (next == state_count) {
                if (state_count == 256) fail("too many composition states");
                states[next].scalar = scalar;
                /* Keep the shortest suffix producing only this pending scalar. */
                unsigned int suffix_length;
                for (suffix_length = 1; suffix_length <= input_length; suffix_length++) {
                    if (convert(decoder, input + input_length - suffix_length, suffix_length, output) == 0
                        && flush(decoder, pending) == 4 && get32(pending) == scalar) break;
                }
                if (suffix_length > input_length || suffix_length > sizeof states[next].prefix)
                    fail("cannot isolate composition state");
                memcpy(states[next].prefix, input + input_length - suffix_length, suffix_length);
                states[next].length = suffix_length;
                state_count++;
            }
            transition[2] = next;
        }
    }
    char path[128];
    snprintf(path, sizeof path, "../src/tables/%s-combining.bin", name);
    FILE *file = fopen(path, "wb");
    if (!file) fail(path);
    put32(file, state_count);
    for (unsigned int state = 0; state < state_count; state++) {
        put32(file, states[state].scalar);
        for (unsigned int byte = 0; byte < 256; byte++)
            for (unsigned int item = 0; item < 3; item++) put32(file, states[state].transitions[byte][item]);
    }
    if (fclose(file) || iconv_close(decoder)) fail("close composition table");
    fprintf(stderr, "%s: %u composition states\n", name, state_count);
}

int main(void) {
    const char *single[] = {
        "ASCII", "ISO-8859-1", "ISO-8859-2", "ISO-8859-3", "ISO-8859-4", "ISO-8859-5",
        "ISO-8859-6", "ISO-8859-7", "ISO-8859-8", "ISO-8859-9", "ISO-8859-10", "ISO-8859-11",
        "ISO-8859-13", "ISO-8859-14", "ISO-8859-15", "ISO-8859-16", "WINDOWS-1250", "WINDOWS-1251",
        "WINDOWS-1252", "WINDOWS-1253", "WINDOWS-1254", "WINDOWS-1255", "WINDOWS-1256", "WINDOWS-1257",
        "WINDOWS-1258", "WINDOWS-874", "TIS-620", "KOI8-R", "KOI8-U", "MACINTOSH", "MAC-CYRILLIC",
        "CP437", "CP850", "CP866", "CP1125"
    };
    const char *multiple[] = {"SHIFT_JIS", "CP932", "EUC-JP", "EUC-KR", "CP949", "GB2312", "GBK",
                              "GB18030", "BIG5", "BIG5-HKSCS"};
    char path[128];
    for (size_t index = 0; index < sizeof single / sizeof *single; index++) {
        snprintf(path, sizeof path, "../src/tables/%s.bin", single[index]);
        generate(single[index], path, 1);
    }
    for (size_t index = 0; index < sizeof multiple / sizeof *multiple; index++) {
        snprintf(path, sizeof path, "../src/tables/%s.bin", multiple[index]);
        generate(multiple[index], path, !strcmp(multiple[index], "EUC-JP") ? 3 : 2);
    }
    generate_gb18030_ranges();
    generate_combining("WINDOWS-1255");
    generate_combining("WINDOWS-1258");
    return 0;
}
