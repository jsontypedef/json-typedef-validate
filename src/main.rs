use anyhow::{Context, Result};
use clap::{crate_version, load_yaml, App, AppSettings};
use jtd::{Schema, ValidateOptions};
use serde::Serialize;
use serde_json::Deserializer;
use std::fs::File;
use std::io::{stdin, BufReader, Read};
use std::process::exit;
use std::borrow::Cow;

fn main() -> Result<()> {
    let cli_yaml = load_yaml!("cli.yaml");
    let matches = App::from(cli_yaml)
        .setting(AppSettings::ColoredHelp)
        .version(crate_version!())
        .get_matches();

    let quiet = matches.is_present("quiet");

    let max_depth = if let Some(s) = matches.value_of("max-depth") {
        s.parse()
            .with_context(|| format!("Failed to parse max depth: {}", s))?
    } else {
        0
    };

    let max_errors = if let Some(s) = matches.value_of("max-errors") {
        s.parse()
            .with_context(|| format!("Failed to parse max errors: {}", s))?
    } else if quiet {
        1
    } else {
        0
    };

    let schema_reader = BufReader::new(match matches.value_of("schema").unwrap() {
        "-" => Box::new(stdin()) as Box<dyn Read>,
        file @ _ => Box::new(File::open(file)?) as Box<dyn Read>,
    });

    let schema = Schema::from_serde_schema(
        serde_json::from_reader(schema_reader).with_context(|| "Failed to parse schema")?,
    )
    .with_context(|| "Malformed schema")?;

    schema.validate().with_context(|| "Invalid schema")?;

    let instance_reader = BufReader::new(match matches.value_of("instances").unwrap() {
        "-" => Box::new(stdin()) as Box<dyn Read>,
        file @ _ => Box::new(File::open(file)?) as Box<dyn Read>,
    });

    let stream = Deserializer::from_reader(instance_reader);
    for instance in stream.into_iter() {
        let instance = instance.with_context(|| format!("Failed to parse instance"))?;

        let errors = jtd::validate(
            &schema,
            &instance,
            ValidateOptions::new().with_max_depth(max_depth).with_max_errors(max_errors),
        )
        .with_context(|| format!("Failed to validate instance"))?;

        if !errors.is_empty() {
            if !quiet {
                // These are the errors we'll output to the user, using the standard
                // JSON Typedef error indicator format.
                let error_indicators: Vec<_> = errors
                    .into_iter()
                    .map(|err| ErrorIndicator {
                        instance_path: to_json_pointer(err.instance_path),
                        schema_path: to_json_pointer(err.schema_path),
                    })
                    .collect();

                for error_indicator in error_indicators {
                    println!("{}", serde_json::to_string(&error_indicator).unwrap());
                }
            }

            exit(1);
        }
    }

    Ok(())
}

#[derive(Serialize)]
struct ErrorIndicator {
    #[serde(rename = "instancePath")]
    instance_path: String,

    #[serde(rename = "schemaPath")]
    schema_path: String,
}

fn to_json_pointer<'a>(path: Vec<Cow<'a, str>>) -> String {
    if path.is_empty() {
        "".to_owned()
    } else {
        format!(
            "/{}",
            path.iter()
                .map(|part| part.replace("~", "~0").replace("/", "~1"))
                .collect::<Vec<_>>()
                .join("/")
        )
    }
}
