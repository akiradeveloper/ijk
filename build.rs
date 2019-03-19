extern crate build_deps;

fn main() {
    build_deps::rerun_if_changed_paths( "behavior/*" ).unwrap();
    build_deps::rerun_if_changed_paths( "behavior" ).unwrap();
}