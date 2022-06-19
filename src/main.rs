extern crate glob;

use anyhow::{Context, Error};
use clap::Parser;
use glob::glob;
use serde_json::Value;
use std::{
  fs,
  path::{Path, PathBuf},
  process::Command,
};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
  #[clap(short, long, value_parser)]
  out_dir: String,

  #[clap(short, long, value_parser)]
  photoprism_dir: String,
}

fn main() -> Result<(), Error> {
  let args = Args::parse();

  let out_dir = Path::new(&args.out_dir).to_path_buf();
  let photoprism_dir = Path::new(&args.photoprism_dir).to_path_buf();

  iterate_files(&photoprism_dir, &out_dir)?;

  Ok(())
}
fn build_new_file_path(image_data: &ImageAndJsonPaths, out_dir: &Path) -> Result<PathBuf, Error> {
  let data = fs::read_to_string(image_data.json_path.as_path())?;
  let json: Value = serde_json::from_str(&data)?;

  // example: "SourceFile": "/photoprism/originals/2009-11-27/004.JPG",
  let source_file = json[0]["SourceFile"]
    .as_str()
    .context("Missing SourceFile json")?;
  let split: Vec<&str> = source_file.split('/').collect();
  let filename = split[split.len() - 1];
  let dir = split[split.len() - 2];

  let new_file_path = out_dir.join(dir).join(filename);
  Ok(new_file_path)
}

fn copy_original_file(image_data: &ImageAndJsonPaths, new_file_path: &Path) -> Result<(), Error> {
  println!(
    "Copying {} to {}",
    image_data.thumbnail_path.to_str().context("no path")?,
    new_file_path.to_str().context("no path")?
  );
  let parent = new_file_path.parent().context("No parent found")?;
  fs::create_dir_all(parent)?;
  fs::copy(image_data.thumbnail_path.as_path(), &new_file_path)?;
  Ok(())
}

#[derive(Debug)]
struct ImageAndJsonPaths {
  thumbnail_path: PathBuf,
  json_path: PathBuf,
}

fn iterate_files(photoprism_dir: &Path, out_dir: &Path) -> Result<(), Error> {
  println!("Reading files...");

  let search_string = photoprism_dir
    .join("cache")
    .join("json")
    .join("**")
    .join("*.json");

  for entry in glob(search_string.to_str().context("no path")?)? {
    let json_path = entry?;
    let thumbnail_path = thumbnail_path_from_json_path(photoprism_dir, &json_path)?;

    if thumbnail_path.exists() {
      let image_data = ImageAndJsonPaths {
        json_path,
        thumbnail_path,
      };

      let new_file_path = build_new_file_path(&image_data, out_dir)?;

      if !new_file_path.exists() {
        copy_original_file(&image_data, &new_file_path)?;
        copy_exif_tags(&image_data.json_path, &new_file_path)?;
      } else {
        println!(
          "Picture has already been copied: {}",
          new_file_path.to_str().context("no path")?
        );
      }
    } else {
      println!(
        "Couldn't find {}",
        thumbnail_path.to_str().context("no path")?
      );
    }
  }

  Ok(())
}

fn thumbnail_path_from_json_path(
  photoprism_dir: &Path,
  json_path: &Path,
) -> Result<PathBuf, Error> {
  let file_prefix_removed = json_path
    .file_name()
    .context("no file_name")?
    .to_str()
    .context("no file string")?
    .split('_') // Remove the _exiftool
    .next()
    .context("no _exiftool found")?
    .split('.') // Remove the
    .next()
    .context("no extension found")?;
  let image_filename = format!("{}_2048x2048_fit.jpg", file_prefix_removed);

  let p3 = json_path.parent().context("no parent")?;
  let p2 = p3.parent().context("no parent")?;
  let p1 = p2.parent().context("no parent")?;
  let out = photoprism_dir
    .join("cache")
    .join("thumbnails")
    .join(p1.file_name().context("not a folder")?)
    .join(p2.file_name().context("not a folder")?)
    .join(p3.file_name().context("not a folder")?)
    .join(image_filename);
  Ok(out)
}

// EXIFTOOL command:
// exiftool -tagsfromfile c03e484261bbdddbb81f6a703e8b5adf4b8b5bac_exiftool.json bhagat-singh_2qmc.jpg
fn copy_exif_tags(json_path: &Path, new_file_path: &Path) -> Result<(), Error> {
  println!(
    "Copying exif tag to {}",
    new_file_path.to_str().context("no file path")?
  );
  Command::new("exiftool")
    .arg("-tagsfromfile")
    .arg(json_path.to_str().context("no file")?)
    .arg(new_file_path.to_str().context("no file")?)
    .arg("-overwrite_original")
    .spawn()?
    .wait()?;

  // Sometimes you'll get a png error, so just consider it a success
  Ok(())
}
