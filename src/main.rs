use anyhow::{format_err, Context, Result};
use clap::{crate_version, App, AppSettings, Arg};
use jtd::{Schema, SerdeSchema, Validator};
use serde::Serialize;
use serde_json::Deserializer;
use std::convert::TryInto;
use std::fs::File;
use std::io::{stdin, BufReader, Read};
use std::process::exit;

fn main() -> Result<()> {
    let matches = App::new("jtd-validate")
        .version(crate_version!())
        .about("Validate JSON data against a JSON Typedef schema")
        .setting(AppSettings::ColoredHelp)
        .arg(
            Arg::with_name("quiet")
                .help("Disable outputting validation errors.")
                .long("quiet")
                .short("q")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("max-depth")
                .help("How many refs to follow recursively before erroring. By default, refs are followed until stack overflow.")
                .long("max-depth")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("max-errors")
                .help("Maximum number of errors to look for in each instance. By default, all validation errors are returned. Specifiying --quiet implies --max-errors=1.")
                .long("max-errors")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("schema")
                .help("File containing the schema to validate against")
                .required(true),
        )
        .arg(
            Arg::with_name("instances")
                .help("Read data to validate (\"instances\") from this file, instead of STDIN"),
        )
        .get_matches();

    // Parse flags as soon as possible, to give users errors if they mistyped
    // something.
    let quiet = matches.is_present("quiet");
    let max_depth: Option<usize> = match matches.value_of("max-depth") {
        Some(s) => Some(
            s.parse()
                .with_context(|| format!("Failed to parse max depth: {}", s))?,
        ),
        None => None,
    };

    let max_errors: Option<usize> = match matches.value_of("max-errors") {
        Some(s) => Some(
            s.parse()
                .with_context(|| format!("Failed to parse max errors: {}", s))?,
        ),
        None => {
            // If the user has specified --quiet, then all they care about is
            // whether there are any validation errors at all. So there's no
            // point in looking for more than one error.
            if quiet {
                Some(1)
            } else {
                None
            }
        }
    };

    // Open up the relevant files, so we can get I/O errors back to the user as
    // soon as possible if the files don't exist or something.
    let schema = BufReader::new(
        File::open(matches.value_of("schema").unwrap())
            .with_context(|| format!("Failed to open schema file"))?,
    );

    let instances: Box<dyn Read> = if let Some(file) = matches.value_of("instances") {
        Box::new(BufReader::new(
            File::open(file).with_context(|| format!("Failed to open instances file"))?,
        ))
    } else {
        Box::new(stdin())
    };

    // Parse the schema input file into a jtd::Schema.
    let serde_schema: SerdeSchema =
        serde_json::from_reader(schema).with_context(|| format!("Failed to parse schema"))?;

    let schema: Schema = serde_schema
        .try_into()
        .map_err(|err| format_err!("invalid schema: {:?}", err))
        .with_context(|| format!("Failed to load schema"))?;

    schema
        .validate()
        .map_err(|err| format_err!("invalid schema: {:?}", err))
        .with_context(|| format!("Failed to verify schema correctness"))?;

    // Construct a validator with the user's supplied max-depth and max-errors,
    // if any.
    let validator = Validator {
        max_depth,
        max_errors,
    };

    // Validate each input from instances against the schema.
    let stream = Deserializer::from_reader(instances);
    for instance in stream.into_iter() {
        let instance = instance.with_context(|| format!("Failed to parse instance"))?;

        let errors = validator
            .validate(&schema, &instance)
            .map_err(|err| format_err!("invalid schema: {:?}", err))
            .with_context(|| format!("Failed to validate instance"))?;

        if !errors.is_empty() {
            if !quiet {
                // These are the errors we'll output to the user, using the standard
                // JSON Typedef error indicator format.
                let error_indicators: Vec<_> = errors
                    .iter()
                    .map(|err| ErrorIndicator {
                        instance_path: to_json_pointer(&err.instance_path),
                        schema_path: to_json_pointer(&err.schema_path),
                    })
                    .collect();

                println!("{}", serde_json::to_string(&error_indicators).unwrap());
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

fn to_json_pointer(path: &[String]) -> String {
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
