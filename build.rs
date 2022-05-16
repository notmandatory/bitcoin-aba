fn main() -> std::io::Result<()> {
    #[cfg(feature = "web-files")]
    {
        use static_files::resource_dir;
        let web_dist_path = "./web/dist";
        resource_dir(web_dist_path).build()?;
    }
    Ok(())
}
