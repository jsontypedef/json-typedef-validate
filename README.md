# jtd-validate: Validate JSON Typedef schemas from the shell

[JSON Type Definition](https://jsontypedef.com), aka
[RFC8927](https://tools.ietf.org/html/rfc8927), is an easy-to-learn,
standardized way to define a schema for JSON data. You can use JSON Typedef to
portably validate data across programming languages, create dummy data, generate
code, and more.

`jtd-validate` is a tool that checks whether an input is valid against a JSON
Typedef schema. It's meant to be convenient to use in shell scripts.

```bash
# user.jtd.json has the following contents:
#
# {
#   "properties": {
#     "name": { "type": "string" }
#   }
# }
echo '{ "name": 123 }' | jtd-validate user.jtd.json | jq
```

```json
{
  "instancePath": "/name",
  "schemaPath": "/properties/name/type"
}
```

## Installation

On macOS, you can install `jtd-validate` via Homebrew:

```bash
brew install jsontypedef/jsontypedef/jtd-validate
```

For all other platforms, you can download and extract the binary yourself from
[the latest
release](https://github.com/jsontypedef/json-typedef-validate/releases/latest).

## Usage

To invoke `jtd-validate`, you need a schema to validate and an input or set of
inputs to validate. Then, you can use either of these invocations:

```bash
# Both of these do the same thing
cat path/to/input_or_inputs.json | jtd-validate path/to/schema.json
jtd-validate path/to/schema.json path/to/input_or_inputs.json
```

`jtd-validate` will do the following things:

1. It will validate each of your input(s) against the schema.
2. If there aren't any validation errors, it will print nothing and exit with
   status code 0.
3. If there are validation errors, it will print each of the validation errors
   on their own line, and exit with status code 1. `jtd-validate` will do this
   on the first bad input it sees.

To customize this behavior, you have a few options:

- The `--max-depth` (`-d`) option sets the maximum number of `ref`s to follow
  during validation (useful if your schema may have cyclical definitions).

- The `--max-errors` (`-e`) option sets the maximum number of errors to return.
  This can improve `jtd-validate`'s performance if your inputs or schemas are
  large.

- The `--quiet` (`-q`) option disables output. This can be useful if you're
  using `jtd-validate` in an `if` in your shell script, such as:

  ```bash
  if jtd-validate -q <(echo "$schema") <(echo "$input"); then
    echo "your input is good"
  else
    echo "your input is not good"
  fi
  ```

  This works thanks to the fact that `jtd-validate` outputs a non-zero status
  code if there are validation errors. The `--quiet` option saves you the
  trouble of redirecting `jtd-validate` to `/dev/null` if you don't want to
  output JSON Typedef validation errors.
