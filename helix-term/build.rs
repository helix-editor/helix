use git2::{DescribeFormatOptions, DescribeOptions, Repository};

fn main() {
    let repo = Repository::open_from_env().unwrap();
    let describe = repo.describe(&DescribeOptions::new()).unwrap();
    let result = describe
        .format(Some(&DescribeFormatOptions::new().dirty_suffix("-dirty")))
        .unwrap();
    println!("cargo:rustc-env=GIT_DESCRIBE={}", result);
    // rerun-if-changed=../.git/HEAD not accurate as we check dirty
    // println!("cargo:rerun-if-changed=../.git/HEAD");
}
