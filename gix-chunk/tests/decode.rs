use gix_chunk::{Id, SENTINEL, file::Index};

#[test]
fn malformed_chunk_tables_are_validation_errors() {
    let cases = [
        (
            "empty index",
            Vec::new(),
            0,
            "Empty chunk indices are not allowed as the point of chunked files is to have chunks.",
        ),
        (
            "truncated table",
            Vec::new(),
            1,
            "The table of contents would be 24 bytes, but got only 0",
        ),
        (
            "early sentinel",
            chunk_file(&[(SENTINEL, 24), (SENTINEL, 26)]),
            1,
            "Sentinel value encountered while processing chunks 0 of 1",
        ),
        (
            "duplicate chunk",
            chunk_file(&[(*b"DATA", 36), (*b"DATA", 37), (SENTINEL, 38)]),
            2,
            "The chunk of kind 'DATA' was encountered more than once",
        ),
        (
            "chunk offset past the file",
            chunk_file(&[(*b"DATA", 27), (SENTINEL, 26)]),
            1,
            "The chunk offset 27 went past the file of length 26 - was it truncated?",
        ),
        (
            "next chunk offset past the file",
            chunk_file(&[(*b"DATA", 24), (SENTINEL, 27)]),
            1,
            "The chunk offset 27 went past the file of length 26 - was it truncated?",
        ),
        (
            "equal offsets",
            chunk_file(&[(*b"DATA", 24), (SENTINEL, 24)]),
            1,
            "All chunk offsets must be incrementing.",
        ),
        (
            "decreasing offsets",
            chunk_file(&[(*b"DATA", 25), (SENTINEL, 24)]),
            1,
            "All chunk offsets must be incrementing.",
        ),
        (
            "missing sentinel",
            chunk_file(&[(*b"DATA", 24), (*b"MISS", 26)]),
            1,
            "Sentinel value wasn't found, saw 'MISS'",
        ),
    ];

    for (case, data, num_chunks, expected_message) in cases {
        let err = Index::from_bytes(&data, 0, num_chunks)
            .err()
            .expect("malformed chunk tables must be rejected");
        assert_eq!(err.to_string(), expected_message, "{case} must retain its diagnostic");
        assert!(err.is_validation(), "{case} must be classified as invalid input: {err}");
    }
}

fn chunk_file(entries: &[(Id, u64)]) -> Vec<u8> {
    let mut data = Vec::new();
    for (kind, offset) in entries {
        data.extend_from_slice(kind);
        data.extend_from_slice(&offset.to_be_bytes());
    }
    data.extend_from_slice(b"ab");
    data
}
