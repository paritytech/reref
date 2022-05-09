use std::{collections::HashSet, fs, path::Path, process::Command};

use anyhow::{anyhow, Result};
use clap::Parser;
use toml_edit::{value, Document};
use walkdir::WalkDir;

#[derive(clap::Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
	#[clap(long)]
	match_git: String,

	#[clap(long)]
	remove_field: String,

	#[clap(long)]
	add_field: String,

	#[clap(long)]
	added_field_value: String,

	#[clap(long)]
	project: String,

	#[clap(long)]
	autocommit: bool,

	#[clap(long)]
	require_field_to_remove: bool,
}

fn main() -> Result<()> {
	let args = Args::parse();

	/*
		It's necessary to verify which packages are actually used in the project by
		its lockfile because usage has shown that a repository might have outdated
		inactive crates which might use remove dependencies
	*/
	let cargo_lock_path = Path::new(&args.project).join("Cargo.lock");
	let lockfile = cargo_lock::Lockfile::load(cargo_lock_path)?;
	let pkgs_in_lockfile: HashSet<String> = {
		HashSet::from_iter(lockfile.packages.iter().filter_map(|pkg| {
			if let Some(src) = pkg.source.as_ref() {
				if src.url().as_str() == args.match_git {
					Some(format!("{}:{}", pkg.name.as_str(), pkg.version))
				} else {
					None
				}
			} else {
				None
			}
		}))
	};

	let mut dependencies_to_update = HashSet::new();

	for entry in WalkDir::new(&args.project) {
		let entry = entry?;

		let cargo_toml_path = entry.path();
		if !cargo_toml_path.ends_with("Cargo.toml") {
			continue;
		}

		let current_cargo_toml = fs::read_to_string(cargo_toml_path)?;
		let mut new_cargo_toml = current_cargo_toml.parse::<Document>()?;
		for (top_level_key, top_level_key_value) in
			new_cargo_toml.clone().iter()
		{
			let dependencies = match top_level_key {
				// https://doc.rust-lang.org/cargo/reference/manifest.html#the-manifest-format
				"dependencies" | "dev-dependencies" | "build-dependencies"
				| "target" => top_level_key_value.as_table().unwrap(),
				_ => continue,
			};

			for (dependency_name, dependency_description) in dependencies.iter()
			{
				let dependency_description =
					if let Some(dependency_description) =
						dependency_description.as_table_like()
					{
						dependency_description
					} else {
						continue;
					};

				let mut matches_target_git = false;
				let mut has_field_to_remove = false;
				for (dependency_attr, dependency_attr_value) in
					dependency_description.iter()
				{
					/*
						[dependencies]
						foo = { git = "https://github.com/org/foo", branch = "master" }
					*/
					if dependency_attr == "git" {
						if dependency_attr_value.as_str()
							== Some(&args.match_git)
						{
							matches_target_git = true;
						}
					} else if dependency_attr == args.remove_field {
						has_field_to_remove = true;
					}
				}

				if matches_target_git {
					if let Some(pkg) = pkgs_in_lockfile.iter().find(|pkg| {
						pkg.starts_with(&format!("{}:", dependency_name))
					}) {
						dependencies_to_update.insert(pkg);

						new_cargo_toml[top_level_key][dependency_name]
							[&args.add_field] = value(args.added_field_value.clone());

						if has_field_to_remove {
							new_cargo_toml[top_level_key][dependency_name]
								.as_table_like_mut()
								.unwrap()
								.remove(&args.remove_field);
						} else if args.require_field_to_remove {
							return Err(anyhow!(
									"Expected [{:?}][{:?}] to have a \"{}\" key in \"{:?}\"",
									top_level_key,
									dependency_name,
									args.remove_field,
									cargo_toml_path
								));
						}
					};
				}
			}
		}

		fs::write(cargo_toml_path, new_cargo_toml.to_string())?;
	}

	println!("dependencies_to_update {:?}", dependencies_to_update);
	Command::new("cargo")
		.arg("update")
		.args(dependencies_to_update.iter().flat_map(|dep| ["-p", dep]))
		.current_dir(&args.project)
		.spawn()?
		.wait()?;

	if args.autocommit {
		Command::new("git")
			.arg("add")
			.arg(".")
			.current_dir(&args.project)
			.spawn()?
			.wait()?;

		Command::new("git")
			.arg("commit")
			.arg("-m")
			.arg(format!(
				"target {{ \"{:?}\" = \"{:?}\" }} for {:?}",
				args.add_field, args.added_field_value, args.match_git,
			))
			.current_dir(&args.project)
			.spawn()?
			.wait()?;
	}

	Ok(())
}
