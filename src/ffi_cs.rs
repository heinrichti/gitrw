use interoptopus::util::NamespaceMappings;
use interoptopus::{Error, Interop};

fn main() -> Result<(), Error> {
    use interoptopus_backend_csharp::overloads::DotNet;
    use interoptopus_backend_csharp::{Config, Generator};

    let config = Config {
        dll_name: "example_library".to_string(),
        namespace_mappings: NamespaceMappings::new("gitrw"),
        ..Config::default()
    };

    Generator::new(config, libgitrw::ffi::ffi_inventory())
        .add_overload_writer(DotNet::new())
        .write_file("bindings/csharp/Interop.cs")?;

    Ok(())
}
