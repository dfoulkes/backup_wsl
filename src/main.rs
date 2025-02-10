use std::process::Command;
use std::error::Error;
use clap::Parser;
use flate2::write::GzEncoder;
use flate2::Compression;
use rayon::prelude::*;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::{env};
use std::fs::{self, File};
use std::sync::mpsc::channel;
use indicatif::{ProgressBar, ProgressStyle};
use num_cpus::get;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(skip)]
    _hidden: (),
}

fn update_progress_bar_message(pb: &ProgressBar, message: &str) {
    pb.set_style(ProgressStyle::default_bar()
        .template(&format!(
            "{{spinner:.green}} [{{elapsed_precise}}] [{{bar:40.cyan/blue}}] {{pos}}/{{len}} ({{eta}})\n{}",
            message
        )).expect("Failed to set progress bar template")
        .progress_chars("#>-"));
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pb = ProgressBar::new(5);

    let temp_dir = env::temp_dir();
    let temp_tar_path = temp_dir.join("ubuntu.tar.gz");
    let output_file_path = PathBuf::from(temp_tar_path);
    // these specs run at roughly 3.2GB worth of memory whilst compressing
    let chunk_size = 100 * 1024 * 1024; // 100MB
    // I know it's hard coded, optimised for my AMD Ryzen 9 7950X3D with a few threads to spare
    //let max_threads = 28;

    // replaced with the use of num_cpus crate to get the number of threads available.
    let max_threads = get();

    update_progress_bar_message(&pb, "Backing up WSL distro 📦");
    pb.inc(1);
    let temp_tar_path = export_wsl_distro()?;
    let file_path = PathBuf::from(temp_tar_path.clone());

    // break the tar up into smaller parts, compress them in parallel, and reassemble them
    update_progress_bar_message(&pb, "Compressing the .tar file to .tar.gz ️🗂️");
    pb.inc(1);
    rayon::ThreadPoolBuilder::new().num_threads(max_threads).build_global().unwrap();
    let parts = split_file(&file_path, chunk_size)?;
    let compressed_files = compress_files_parallel(parts, max_threads)?;
    reassemble_compressed_blocks(compressed_files, &output_file_path)?;

    // move the tar.gz file to the final location and manage the process to avoid data loss
    update_progress_bar_message(&pb, "Moving .tar.gz to final location");
    pb.inc(1);
    manage_output_dir()?;
    move_tar_gz_file(&output_file_path)?;
    delete_bck_file()?;

    // finally, present a user message to say the job is complete.
    pb.inc(1);
    pb.finish_with_message("Backup complete 🎉");
    Ok(())
}

fn export_wsl_distro() -> Result<PathBuf, Box<dyn Error>> {
    let temp_dir = env::temp_dir();
    let temp_tar_path = temp_dir.join("ubuntu.tar");
    let backup_command = format!("wsl --export Ubuntu {}", temp_tar_path.to_string_lossy());
    Command::new("cmd").args(&["/C", &backup_command]).output()?;
    Ok(temp_tar_path)
}

fn split_file(file_path: &PathBuf, chunk_size: usize) -> io::Result<Vec<PathBuf>> {
    let mut file = File::open(file_path)?;
    let mut parts = Vec::new();
    let mut part_number = 0;

    loop {
        let mut buffer = vec![0; chunk_size];
        let bytes_read = file.read(&mut buffer)?;
        //break out of loop if no more bytes to read
        if bytes_read == 0 {
            break;
        }

        let part_path = file_path.with_file_name(format!("part_{}", part_number));
        let mut part_file = File::create(&part_path)?;
        part_file.write_all(&buffer[..bytes_read])?;
        parts.push(part_path);
        part_number += 1;
    }

    Ok(parts)
}

fn compress_files_parallel(files: Vec<PathBuf>, _max_threads: usize) -> io::Result<Vec<PathBuf>> {
    let (sender, receiver) = channel();
    let files_len = files.len(); // Capture the length of `files` before it's moved
    files.into_par_iter().enumerate().for_each_with(sender, |s, (index, file_path)| {
        let compressed_file_path = file_path.with_extension("gz");
        let file = File::open(&file_path).unwrap();
        let compressed_file = File::create(&compressed_file_path).unwrap();
        let mut encoder = GzEncoder::new(compressed_file, Compression::best());
        let mut buffer = Vec::new();
        file.take(usize::MAX as u64).read_to_end(&mut buffer).unwrap();
        encoder.write_all(&buffer).unwrap();
        encoder.finish().unwrap();
        fs::remove_file(file_path).unwrap();
        s.send((index, compressed_file_path)).unwrap();
    });

    // Use `files_len` here instead of trying to access `files.len()`
    let mut compressed_files: Vec<_> = receiver.iter().take(files_len).collect();
    compressed_files.sort_by_key(|k| k.0);
    Ok(compressed_files.into_iter().map(|(_, path)| path).collect())
}

fn reassemble_compressed_blocks(compressed_files: Vec<PathBuf>, output_file_path: &PathBuf) -> io::Result<()> {
    let mut output_file = File::create(output_file_path)?;

    for path in &compressed_files {
        let mut input_file = File::open(path)?;
        io::copy(&mut input_file, &mut output_file)?;
    }

    // Cleanup: Remove the temporary compressed part files
    for path in compressed_files {
        if path.exists() {
            fs::remove_file(path)?;
        }
    }

    Ok(())
}

fn move_tar_gz_file(tar_gz_path: &PathBuf) -> Result<(), Box<dyn Error>> {
    let final_path = PathBuf::from("P:\\wsl\\backup\\ubuntu.tar.gz");
    let reference_for_later_delete = tar_gz_path.clone();
    fs::copy(tar_gz_path, final_path)?;
    fs::remove_file(reference_for_later_delete)?;
    Ok(())
}

//fn check if output files exists, if it does then copy it to P:\\wsl\\backup\\ubuntu.tar.gz.bck
fn manage_output_dir() -> Result<(), Box<dyn Error>> {
    let final_path = PathBuf::from("P:\\wsl\\backup\\ubuntu.tar.gz");
    let backup_path = PathBuf::from("P:\\wsl\\backup\\ubuntu.tar.gz.bck");
    if final_path.exists() {
        fs::copy(final_path, backup_path)?;
    }
    Ok(())
}

fn delete_bck_file() -> Result<(), Box<dyn Error>> {
    let backup_path = PathBuf::from("P:\\wsl\\backup\\ubuntu.tar.gz.bck");
    if backup_path.exists() {
        fs::remove_file(backup_path)?;
    }
    Ok(())
}