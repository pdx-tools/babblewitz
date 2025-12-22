use anyhow::Result;
use std::path::Path;
use std::process::Command;

pub fn sync_assets() -> Result<()> {
    check_rclone_available()?;
    let local_assets_dir = Path::new("corpus").join("saves");

    // List of known asset files to sync
    let asset_files = vec![
        "ck3/autosave.ck3",
        "eu4/autosave.eu4",
        "hoi4/canada.hoi4",
        "imperator/autosave.rome",
        "stellaris/test.sav",
        "vic3/autosave.v3",
    ];

    sync_specific_assets_with_rclone(&local_assets_dir, &asset_files)?;

    println!("✅ Asset sync completed!");
    Ok(())
}

fn check_rclone_available() -> Result<()> {
    match Command::new("rclone").arg("version").output() {
        Ok(output) if output.status.success() => {
            Ok(())
        }
        _ => Err(anyhow::anyhow!(
            "❌ rclone command failed. Please ensure rclone is properly installed and available in PATH\n
             \n
             rclone is required in order to fetch large save files from the cloud."
        )),
    }
}

fn sync_specific_assets_with_rclone(local_dir: &Path, asset_files: &[&str]) -> Result<()> {
    use std::process::Stdio;

    // Use copyto for each file individually to avoid any listing operations
    for file_path in asset_files {
        let local_path = file_path;
        let local_file = local_dir.join(local_path);

        // Create local directory if it doesn't exist
        let parent = local_file.parent().expect("parent directory");
        std::fs::create_dir_all(parent)?;

        println!("📄 Syncing {}", file_path);

        let status = Command::new("rclone")
            .arg("copyto")
            .arg("--s3-provider=AWS")
            .arg("--s3-endpoint")
            .arg("s3.us-west-002.backblazeb2.com")
            .arg("--s3-no-check-bucket")
            .arg("--s3-decompress")
            .arg("--log-level")
            .arg("ERROR")
            .arg(format!(":s3:pdx-tools-public/babblewitz/{}", file_path))
            .arg(&local_file)
            .arg("--progress")
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        if !status.success() {
            println!("⚠️  Could not sync {}", file_path);
        } else {
            println!("✅ Synced {}", file_path);
        }
    }

    Ok(())
}
