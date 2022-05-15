#[cfg(feature = "server")]
use static_files::resource_dir;

fn main() -> std::io::Result<()> {
    #[cfg(feature = "server")]
    resource_dir("./web/dist").build()?;

    Ok(())
}
