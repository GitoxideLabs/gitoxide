use gix_hash::ObjectId;
use std::{collections::HashMap, path::PathBuf};

pub use gix_testtools::Result;

static SHA1_TO_SHA256_HASHES: std::sync::LazyLock<HashMap<&str, &str>> = std::sync::LazyLock::new(|| {
    [
        (
            "85df34aa34848b8138b2b3dcff5fb5c2b734e0ce",
            "f459aba048cd6ff7351ab93975d196a8d60ce284aed4aa34d75b12aba0a35824",
        ),
        (
            "62ed296d9986f50477e9f7b7e81cd0258939a43d",
            "3742cabb5eba4355c204407e1ae589e65ae7cff4afad74a3296fac966847db9c",
        ),
        (
            "fe63a8a9fb7c27c089835aae92cbda675523803a",
            "29fa86a3c0a765afc2a5d702f99e1a0c7921a80bdfb6fa01e9abb01e0d81839e",
        ),
        (
            "288e509293165cb5630d08f4185bdf2445bf6170",
            "10d977a4ec5ad49aff689ea886dd430f5cf2f618811521fa0df2ccb8c69811ae",
        ),
        (
            "9902e3c3e8f0c569b4ab295ddf473e6de763e1e7",
            "bbaf9640a7404a15394dae2606c5090cb44a722be2167d9d78485779aaf4e065",
        ),
        (
            "bcb05040a6925f2ff5e10d3ae1f9264f2e8c43ac",
            "3e87837a4730030fb4a6bf72d121d4e6305df46e7fd93d1f4f3f1b22845112c7",
        ),
        (
            "134385f6d781b7e97062102c6a483440bfda2a03",
            "5c4c31e0551f0d1fb410b7b9366604b050ea3388b96885063f10ba4c3e2dedd0",
        ),
        (
            "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391",
            "473a0f4c3be8a93681a267e3b1e9a7dcda1185436fe141f7749120a303721813",
        ),
        (
            "f1cce1b5c7efcdfa106e95caa6c45a2cae48a481",
            "8a6ffed7d2ff3f19aca13dfb7a8cfd7c100a59d56fd615c5df3e0fd04852b855",
        ),
        (
            "3be0c4c793c634c8fd95054345d4935d10a0879a",
            "c6326f70751d8a66f6969944d7379ea0db4bc2b397ed854e9671ca4b298e1ff1",
        ),
        (
            "496d6428b9cf92981dc9495211e6e1120fb6f2ba",
            "5f6f307bcc469c02acba4f7da42d8d4defdda8209777fe732956f1e2fa0db3ff",
        ),
        (
            "4277b6e69d25e5efa77c455340557b384a4c018a",
            "1b71a65f9ac3dffcea1bd42639e5d38f44b0df207b0e16c44c49aaed84bb24b0",
        ),
        (
            "70fb16fc77b03e16acb4a5b1a6caf79ba302919a",
            "c742c0863c1f0443ea3a0fbe41ef215d48f12008ca2349bd0764909dbffc8650",
        ),
        (
            "722bf6b8c3d9e3a11fa5100a02ed9b140e1d209c",
            "1def6ae21303f601aeec8546e5e4938128e9194405a5e7540d2947f3c580455f",
        ),
        (
            "d09384f312b03e4a1413160739805ff25e8fe99d",
            "dceec07c165522093aa46f4ef352a7bc38a0d25961aadab6c70e94a1a9898ddd",
        ),
        (
            "01ec18a3ebf2855708ad3c9d244306bc1fae3e9b",
            "fb6f3cf687f7adc3da7d030935d071b738861741046d030b37e5efcc9cde5131",
        ),
        (
            "1a27cb1a26c9faed9f0d1975326fe51123ab01ed",
            "314fa631c39ac422ce23793fd4ffbccbf31beb2045f9b45a5b021db2fb5c26ee",
        ),
        (
            "2083b02a78e88b747e305b6ed3d5a861cf9fb73f",
            "f5ccd16930bae6234f119f676616d55f936ee291139e1842b6fd16de1fe0a5e7",
        ),
        (
            "33eb18340e4eaae3e3dcf80222b02f161cd3f966",
            "b7c4b48a4121b8a1444b3023a7754a664e71b62eb1b9a8a56c80b5915b845a14",
        ),
        (
            "5805b676e247eb9a8046ad0c4d249cd2fb2513df",
            "e822bc1ea19cfd0f1a4d5ed57b2a6eae55553eb33af1a663fb0f6ec32e144581",
        ),
        (
            "e07cf1277ff7c43090f1acfc85a46039e7de1272",
            "bf186f330f50b0cfb2daaa1d470644ca4df96b02c9bac6b512d636f047dbca87",
        ),
        (
            "eeab3243aad67bc838fc4425f759453bf0b47785",
            "090b8b537ef00eebd57f5e3c78a8baab77c8d90639566ab738f1e61985a428b1",
        ),
        (
            "22fbc169eeca3c9678fc7028aa80fad5ef49019f",
            "1211d19c84d684fe270709c5c2e33a99a82a0bf96caf0bb2f2af5b3532f9e8cb",
        ),
        (
            "302a5d0530ec688c241f32c2f2b61b964dd17bee",
            "611e887cff25810f4fba38c8160a3bdb39909e968280cd03622fa8974d270ed9",
        ),
        (
            "693c775700cf90bd158ee6e7f14dd1b7bd83a4ce",
            "e2149038ff4573b2e576f87f55aadcf7a887979c5d97057368d617f583ffd6ec",
        ),
        (
            "9556057aee5abb06912922e9f26c46386a816822",
            "9a3e230fc8479e41397b78b9295510e38be525ec05a08c1ceb797547dc93ed4c",
        ),
        (
            "17d78c64cef6c33a10a604573fd2c429e477fd63",
            "e47e1df5636110feefb5b858c346dbd1c0feebfc37651a238ec5a6300ed2f666",
        ),
        (
            "945d8a360915631ad545e0cf04630d86d3d4eaa1",
            "ee5b287f2a3e4efdb3d051dc2920420fe565c8ad52bd073c426ee7a395f512a8",
        ),
        (
            "a863c02247a6c5ba32dff5224459f52aa7f77f7b",
            "921f464bad45b282ad3d3953d9727a2c233ccc3de3253b1d129f0824d06ef686",
        ),
        (
            "2f291881edfb0597493a52d26ea09dd7340ce507",
            "23773b3116a9e07594f6f8d549667981bf49b8ba8fd7e3c0f5722e8b785dbb79",
        ),
        (
            "9c46b8765703273feb10a2ebd810e70b8e2ca44a",
            "91153ec24a9bd8c4f98e6e217973c6edcde22dd4e2b1200ec4d04caeb809e710",
        ),
        (
            "fb3e21cf45b04b617011d2b30973f3e5ce60d0cd",
            "c91ae6d1f752ba52df944b7ba1cbb0885f02c753da284c5513907fd765613f79",
        ),
        (
            "efd9a841189668f1bab5b8ebade9cd0a1b139a37",
            "0fc125d0690528eeff91d75edb3da0fa7bf75ed8eca44c0e402d4a6b6975e86a",
        ),
        (
            "ce2e8ffaa9608a26f7b21afc1db89cadb54fd353",
            "5d9bb5ee5204e19d5b5c3d4f51807e4429972f7871965ee1673edb1c196721f8",
        ),
        (
            "9152eeee2328073cf23dcf8e90c949170b711659",
            "9b1336395000ea1dda99a04bec4ef7d4eeea969312ec4d2fa86b6527bfd8fbfd",
        ),
        (
            "ad33ff2d0c4fc77d56b5fbff6f86f332fe792d83",
            "16dd9ca7a213dd00c9613d353ef619b29b4f566c64265b3818357a1a5048d8be",
        ),
        (
            "0edb95c0c0d9933d88f532ec08fcd405d0eee882",
            "1ef3045172ca9520015ead122a3d8f4a729567f4290d2dd3626950679e7e52bb",
        ),
        (
            "33aa07785dd667c0196064e3be3c51dd9b4744ef",
            "75e848e191e09b344b8b7b21f84e3f139723df091d7c1294a034d737b7d5bd0c",
        ),
        (
            "f49838d84281c3988eeadd988d97dd358c9f9dc4",
            "5db834abbc0bc7f4d56b2375bbe5095640b05b8e0a817c18798fb30b55c1163d",
        ),
        (
            "48e8dac19508f4238f06c8de2b10301ce64a641c",
            "a9e888378d56d411a71b97e18ab3a4ee4a8267eae3f81550ef43b3e49a611944",
        ),
        (
            "66a309480201c4157b0eae86da69f2d606aadbe7",
            "f20b45b1316fc7f3f2736c771a983f2e003b029e301a3270915fe20831fcc6ec",
        ),
        (
            "b5665181bf4c338ab16b10da0524d81b96aff209",
            "d38fb6442915c8133c90f79d70a0bd235d2d01e5271e756d6661f06cb890eaf8",
        ),
        (
            "94cf3f3a4c782b672173423e7a4157a02957dd48",
            "176d48056d4bd0af651ffd0686723059fe0acdbb85ca16fbe0d58f693e7e2f94",
        ),
        (
            "80947acb398362d8236fcb8bf0f8a9dac640583f",
            "c981468050bd19c62b0e0a9d14b54cb21817fde64b27b8867b850d4325a0f4f9",
        ),
        (
            "34e5ff5ce3d3ba9f0a00d11a7fad72551fff0861",
            "70844651e0c4091ceb69c98193b569fc982393c9ea9f08ca2c6dea3119814076",
        ),
        (
            "58912d92944087dcb09dca79cdd2a937cc158bed",
            "5465e8974bf0a50bf4dc79dc7e987a9c2276404baadcd29880b4051068e32bcb",
        ),
        (
            "2dce37be587e07caef8c4a5ab60b423b13a8536a",
            "faf7e7d36b212dc6ce1a20a8341268f8e5603013f58630511d64288bef7840fb",
        ),
        (
            "0f6632a5a7d81417488b86692b729e49c1b73056",
            "786016fb4d46d6ecd6e6dd1b676df313b255ed8476b5582e12d3fb664f265a0e",
        ),
        (
            "a9c28710e058af4e5163699960234adb9fb2abc7",
            "677f406004680360df4abcf949ab86ce4236342ac2acb04ddbc732172ad5cdd3",
        ),
        (
            "77fd3c6832c0cd542f7a39f3af9250c3268db979",
            "ec2158976fa140544b45d03880203e2096d57ddf92d39d8390e834a39088253a",
        ),
        (
            "b648f955b930ca95352fae6f22cb593ee0244b27",
            "183317adb48684f76d33462ccae9ca0a89a88c48509bb73ca42a910d146477ad",
        ),
        (
            "65d6af66f60b8e39fd1ba6a1423178831e764ec5",
            "c0a25d64fa9426c62563cf5359cf551b69f8c561d5199ba40a79147c1da757ed",
        ),
        (
            "8cb5f13b66ce52a49399a2c49f537ee2b812369c",
            "06ec706b9c479d9fc922c31604e88d9b54a38a430d0dbd3c4ae67ba7d2b4162a",
        ),
        (
            "cb6a6befc0a852ac74d74e0354e0f004af29cb79",
            "3dca66a931b51c6a47e83b0a4d423b1774c8d3360683c80cb736c81be5c3538b",
        ),
    ]
    .into()
});

/// Convert a hexadecimal hash into its corresponding `ObjectId` or _panic_.
pub fn hex_to_id(hex: &str) -> ObjectId {
    match gix_testtools::object_hash_from_env().unwrap_or_default() {
        gix_hash::Kind::Sha1 => ObjectId::from_hex(hex.as_bytes()).expect("40 bytes hex"),
        gix_hash::Kind::Sha256 => ObjectId::from_hex(
            SHA1_TO_SHA256_HASHES
                .get(hex)
                .unwrap_or_else(|| panic!("SHA-1 {hex} wasn't mapped to SHA-256 yet"))
                .as_bytes(),
        )
        .expect("64 bytes hex"),
        _ => unimplemented!(),
    }
}

/// Get the path to a fixture directory from a script that creates a single repository.
pub fn fixture(script_name: &str) -> Result<PathBuf> {
    crate::scripted_fixture_read_only(script_name)
}

/// Get an object database handle for `objects_dir`, respecting the hash kind configured for tests.
pub fn odb_at(objects_dir: impl Into<PathBuf>) -> Result<gix_odb::Handle> {
    Ok(gix_odb::at_opts(
        objects_dir,
        Vec::new(),
        gix_odb::store::init::Options {
            object_hash: gix_testtools::object_hash_from_env().unwrap_or_default(),
            ..Default::default()
        },
    )?)
}

/// Get an object database handle from a fixture script that creates a single repository.
pub fn fixture_odb(script_name: &str) -> Result<gix_odb::Handle> {
    let dir = fixture(script_name)?;
    odb_at(dir.join(".git").join("objects"))
}

/// Get a fixture path and object database for a named sub-repository within a fixture.
pub fn named_fixture(script_name: &str, repo_name: &str) -> Result<(PathBuf, gix_odb::Handle)> {
    let dir = fixture(script_name)?;
    let repo_dir = dir.join(repo_name);
    let odb = odb_at(repo_dir.join(".git").join("objects"))?;
    Ok((repo_dir, odb))
}

/// Load a commit graph if available for the given object store.
pub fn commit_graph(store: &gix_odb::Store) -> Option<gix_commitgraph::Graph> {
    gix_commitgraph::at(store.path().join("info")).ok()
}

/// Execute `git log --oneline --graph --decorate --all` in the given repository
/// and return the output as a string. Useful for snapshot testing.
pub fn git_graph(repo_dir: impl AsRef<std::path::Path>) -> Result<String> {
    git_graph_internal(repo_dir, false)
}

/// Like `git_graph`, but includes commit timestamps (Unix epoch seconds).
/// Use this for tests where commit ordering depends on time.
pub fn git_graph_with_time(repo_dir: impl AsRef<std::path::Path>) -> Result<String> {
    git_graph_internal(repo_dir, true)
}

fn git_graph_internal(repo_dir: impl AsRef<std::path::Path>, with_time: bool) -> Result<String> {
    use gix_object::bstr::{ByteSlice, ByteVec};
    let format = if with_time {
        "--pretty=format:%H %ct%d %s"
    } else {
        "--pretty=format:%H %d %s"
    };
    let out = std::process::Command::new(gix_path::env::exe_invocation())
        .current_dir(repo_dir)
        .args(["log", "--oneline", "--graph", "--decorate", "--all", format])
        .output()?;
    if !out.status.success() {
        return Err(format!("git log failed: {err}", err = out.stderr.to_str_lossy()).into());
    }
    Ok(gix_testtools::normalize_hashes(&out.stdout.into_string_lossy()).0)
}

/// Parse commit names to IDs from git log output.
/// Returns a map of commit message (first word) to ObjectId.
pub fn parse_commit_names(repo_path: &std::path::Path) -> Result<std::collections::HashMap<String, ObjectId>> {
    let output = std::process::Command::new("git")
        .current_dir(repo_path)
        .args(["log", "--all", "--format=%H %s"])
        .output()?;
    let mut commits = std::collections::HashMap::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let mut parts = line.split_whitespace();
        if let (Some(hash), Some(name)) = (parts.next(), parts.next()) {
            commits.insert(name.to_string(), gix_hash::ObjectId::from_hex(hash.as_bytes())?);
        }
    }
    Ok(commits)
}

/// Run `git rev-list` with the given arguments and return the resulting commit IDs.
/// Useful for verifying traversal results against git's baseline behavior.
pub fn git_rev_list(repo_path: &std::path::Path, args: &[&str]) -> Result<Vec<ObjectId>> {
    let output = std::process::Command::new("git")
        .current_dir(repo_path)
        .arg("rev-list")
        .args(args)
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| gix_hash::ObjectId::from_hex(s.trim().as_bytes()).expect("valid hash"))
        .collect())
}
