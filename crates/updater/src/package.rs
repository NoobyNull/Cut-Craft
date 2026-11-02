//! A downloaded release: checked against GitHub's checksum, the program taken out of its
//! archive, and swapped in for the running one.

use sha2::{Digest as _, Sha256};
use std::fmt::Write as _;
use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};

/// Whether `bytes` are the file GitHub published: its `sha256:<hex>` checksum. A download
/// with no checksum is refused, since nothing says it arrived whole and unchanged.
pub fn verify(bytes: &[u8], digest: Option<&str>) -> Result<(), String> {
    let Some(want) = digest.and_then(|d| d.strip_prefix("sha256:")) else {
        return Err(
            "GitHub has no checksum for this download, so Cut Craft cannot check it \
                    arrived whole. Download it from the release page instead."
                .to_owned(),
        );
    };
    let got = Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut hex, b| {
            let _ = write!(hex, "{b:02x}");
            hex
        });
    if got.eq_ignore_ascii_case(want) {
        Ok(())
    } else {
        Err(
            "The download is damaged or was changed on the way (its checksum does not \
             match GitHub's). Nothing was installed. Try again later."
                .to_owned(),
        )
    }
}

/// Whether `bytes` were signed with the release key `key` (a minisign public key file):
/// `signature` is the `.minisig` file published beside the download, and its trusted
/// comment must name the download (`file:<name>`), so an older signed file cannot be passed
/// off as a newer one.
pub fn verify_signature(
    bytes: &[u8],
    signature: &str,
    key: &str,
    name: &str,
) -> Result<(), String> {
    let refused = |why: &str| {
        format!(
            "The download's signature {why}, so it may not be a real Cut Craft release. \
             Nothing was installed."
        )
    };
    let key = key
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with("untrusted comment:"))
        .and_then(|line| minisign_verify::PublicKey::from_base64(line).ok())
        .ok_or(
            "This copy of Cut Craft has no release key built in, so it cannot check \
                downloads. Download the new version from the release page instead.",
        )?;
    let signature =
        minisign_verify::Signature::decode(signature).map_err(|_| refused("could not be read"))?;
    key.verify(bytes, &signature, false)
        .map_err(|_| refused("does not match"))?;
    let named = signature
        .trusted_comment()
        .split_whitespace()
        .any(|word| word.strip_prefix("file:") == Some(name));
    if named {
        Ok(())
    } else {
        Err(refused(&format!("is for another file than {name}")))
    }
}

/// The program `name` (`cut-craft` or `cut-craft.exe`) inside a release archive.
pub fn program_from(archive: &[u8], zip: bool, name: &str) -> Result<Vec<u8>, String> {
    let missing = || format!("the download has no {name} in it");
    let named = |path: &Path| path.file_name().is_some_and(|f| f == name);
    if zip {
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(archive))
            .map_err(|e| format!("the download is not a readable .zip ({e})"))?;
        let index = (0..z.len())
            .find(|&i| {
                z.by_index(i)
                    .ok()
                    .and_then(|f| f.enclosed_name())
                    .is_some_and(|p| named(&p))
            })
            .ok_or_else(missing)?;
        let mut out = vec![];
        z.by_index(index)
            .and_then(|mut f| f.read_to_end(&mut out).map_err(Into::into))
            .map_err(|e| format!("could not unpack {name} ({e})"))?;
        Ok(out)
    } else {
        let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(archive));
        let unreadable =
            |e: std::io::Error| format!("the download is not a readable .tar.gz ({e})");
        for entry in tar.entries().map_err(unreadable)? {
            let mut entry = entry.map_err(unreadable)?;
            if entry.header().entry_type().is_file() && named(&entry.path().map_err(unreadable)?) {
                let mut out = vec![];
                entry.read_to_end(&mut out).map_err(unreadable)?;
                return Ok(out);
            }
        }
        Err(missing())
    }
}

/// `target` with `.ext` added to its whole name: `cut-craft.exe` → `cut-craft.exe.old`.
fn beside(target: &Path, ext: &str) -> PathBuf {
    let mut name = target.file_name().unwrap_or_default().to_os_string();
    name.push(ext);
    target.with_file_name(name)
}

/// Where the program before an update is kept, until the next start removes it.
pub fn backup_of(target: &Path) -> PathBuf {
    beside(target, ".old")
}

/// Puts `program` where `target` is. The running program is renamed aside first (Windows
/// allows renaming a running program, not overwriting it); if the new one cannot be put in
/// its place, the old one is put back.
pub fn replace(target: &Path, program: &[u8]) -> Result<(), String> {
    let dir = target.parent().unwrap_or(Path::new("."));
    let cannot = |e: std::io::Error| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            format!(
                "Cut Craft is not allowed to change its own files in {}. It was probably \
                 installed for every user of this computer: download the new version from \
                 the release page and install it the same way.",
                dir.display()
            )
        } else {
            format!("could not install the update in {} ({e})", dir.display())
        }
    };
    let new = beside(target, ".new");
    fs::write(&new, program).map_err(cannot)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        fs::set_permissions(&new, fs::Permissions::from_mode(0o755)).map_err(cannot)?;
    }
    let old = backup_of(target);
    // an earlier update's left-over; on Windows it may still be running, which is fine
    let _ = fs::remove_file(&old);
    if let Err(e) = fs::rename(target, &old) {
        let _ = fs::remove_file(&new);
        return Err(cannot(e));
    }
    if let Err(e) = fs::rename(&new, target) {
        let _ = fs::rename(&old, target);
        let _ = fs::remove_file(&new);
        return Err(cannot(e));
    }
    Ok(())
}

/// Removes what [`replace`] set aside last time, once the new program is the one running.
pub fn clean_up(target: &Path) {
    for left in [backup_of(target), beside(target, ".new")] {
        if left.exists() && fs::remove_file(&left).is_ok() {
            log::info!("removed {} from the last update", left.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    #[test]
    fn a_download_must_match_its_checksum() {
        // printf hello | sha256sum
        let hello = "sha256:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
        assert_eq!(verify(b"hello", Some(hello)), Ok(()));
        assert_eq!(
            verify(
                b"hello",
                Some(&hello.to_uppercase().replace("SHA256", "sha256"))
            ),
            Ok(())
        );
        assert!(
            verify(b"hellO", Some(hello))
                .unwrap_err()
                .contains("Nothing was installed")
        );
        assert!(verify(b"hello", None).unwrap_err().contains("no checksum"));
        assert!(verify(b"hello", Some("md5:abc")).is_err());
    }

    /// A key pair and a signature made as `minisign -S -t "file:<name>"` makes them.
    fn sign(bytes: &[u8], comment: &str) -> (String, String) {
        let pair = minisign::KeyPair::generate_unencrypted_keypair().unwrap();
        let signature = minisign::sign(
            Some(&pair.pk),
            &pair.sk,
            std::io::Cursor::new(bytes),
            Some(comment),
            None,
        )
        .unwrap();
        (pair.pk.to_box().unwrap().to_string(), signature.to_string())
    }

    #[test]
    fn a_download_must_be_signed_with_the_release_key() {
        let name = "cut-craft-v0.5.0-linux-x86_64.tar.gz";
        let (key, signature) = sign(b"program", &format!("file:{name}"));
        assert_eq!(verify_signature(b"program", &signature, &key, name), Ok(()));
        let e = verify_signature(b"programme", &signature, &key, name).unwrap_err();
        assert!(e.contains("does not match"), "{e}");
        // signed by someone else's key
        let (other, _) = sign(b"program", "x");
        assert!(verify_signature(b"program", &signature, &other, name).is_err());
        assert!(verify_signature(b"program", "junk", &key, name).is_err());
        let none = verify_signature(b"program", &signature, "untrusted comment: none\n", name);
        assert!(none.unwrap_err().contains("no release key"));
    }

    #[test]
    fn an_older_signed_download_cannot_pass_as_a_newer_one() {
        let old = "cut-craft-v0.4.9-linux-x86_64.tar.gz";
        let (key, signature) = sign(b"old program", &format!("file:{old}"));
        let e = verify_signature(
            b"old program",
            &signature,
            &key,
            "cut-craft-v0.5.0-linux-x86_64.tar.gz",
        )
        .unwrap_err();
        assert!(e.contains("another file"), "{e}");
        // the name must be the whole word, not a part of it
        let (key, signature) = sign(b"p", "file:x.tar.gz.old");
        assert!(verify_signature(b"p", &signature, &key, "x.tar.gz").is_err());
    }

    fn tar_gz(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut tar = tar::Builder::new(flate2::write::GzEncoder::new(
            vec![],
            flate2::Compression::fast(),
        ));
        for (path, data) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            tar.append_data(&mut header, path, *data).unwrap();
        }
        tar.into_inner().unwrap().finish().unwrap()
    }

    fn zip(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut z = zip::ZipWriter::new(std::io::Cursor::new(vec![]));
        for (path, data) in files {
            z.start_file(*path, zip::write::SimpleFileOptions::default())
                .unwrap();
            z.write_all(data).unwrap();
        }
        z.finish().unwrap().into_inner()
    }

    #[test]
    fn takes_the_program_out_of_a_linux_download() {
        let archive = tar_gz(&[
            ("cut-craft-v0.4.4-linux-x86_64/README.md", b"readme"),
            ("cut-craft-v0.4.4-linux-x86_64/cut-craft", b"\x7fELF new"),
        ]);
        assert_eq!(
            program_from(&archive, false, "cut-craft").unwrap(),
            b"\x7fELF new"
        );
        assert!(program_from(&archive, false, "cut-craft.exe").is_err());
        assert!(program_from(b"not gzip", false, "cut-craft").is_err());
    }

    #[test]
    fn takes_the_program_out_of_a_windows_download() {
        let archive = zip(&[
            ("cut-craft-v0.4.4-windows-x86_64/README.md", b"readme"),
            ("cut-craft-v0.4.4-windows-x86_64/cut-craft.exe", b"MZ new"),
        ]);
        assert_eq!(
            program_from(&archive, true, "cut-craft.exe").unwrap(),
            b"MZ new"
        );
        assert!(
            program_from(&archive, true, "cut-craft")
                .unwrap_err()
                .contains("no cut-craft")
        );
    }

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cut-craft-updater-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn swaps_the_program_and_keeps_the_old_one_until_next_start() {
        let dir = scratch("swap");
        let exe = dir.join("cut-craft.exe");
        fs::write(&exe, b"old").unwrap();
        replace(&exe, b"new").unwrap();
        assert_eq!(fs::read(&exe).unwrap(), b"new");
        assert_eq!(fs::read(backup_of(&exe)).unwrap(), b"old");
        assert!(!dir.join("cut-craft.exe.new").exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = fs::metadata(&exe).unwrap().permissions().mode();
            assert_eq!(mode & 0o111, 0o111, "the new program can be run");
        }
        // a second update replaces the earlier backup
        replace(&exe, b"newer").unwrap();
        assert_eq!(fs::read(backup_of(&exe)).unwrap(), b"new");
        clean_up(&exe);
        assert!(!backup_of(&exe).exists());
        assert_eq!(fs::read(&exe).unwrap(), b"newer");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_missing_program_is_not_replaced() {
        let dir = scratch("missing");
        let exe = dir.join("cut-craft");
        assert!(replace(&exe, b"new").is_err());
        assert!(!exe.exists());
        assert!(!dir.join("cut-craft.new").exists(), "nothing left behind");
        fs::remove_dir_all(dir).unwrap();
    }
}
