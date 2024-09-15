fn main() {
    cxx_build::bridge("src/main.rs")
        .file("src/webserver.cpp")
        .std("c++14")
        .compile("test-dashboard");

    // println!("cargo:rerun-if-changed=src/main.rs");
    // println!("cargo:rerun-if-changed=src/webserver.cpp");
    // println!("cargo:rerun-if-changed=src/webserver.h");
    // println!("I am just a curious cat");
    // panic!("no");
}
