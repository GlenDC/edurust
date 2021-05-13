# EduRust

Are the tests passing?  
[![Continuous integration Actions Status](https://github.com/GlenDC/edurust/workflows/Continuous%20integration/badge.svg)](https://github.com/GlenDC/edurust/actions)

A repo containing small projects that were build by the author,
with as only purpose to get comfortable with Rust and Experiment with it.

## Debug Rust & VSCode

How do you debug rust with vscode?

On macOS get:

- CodeLLDB extension
- rust official extension

Use this as your `launch.json` file:

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "name": "Launch webservice",
      "type": "lldb",
      "request": "launch",
      "program": "${workspaceRoot}/target/debug/webservice",
      "args": [],
      "console": "internalConsole",
      "cwd": "${workspaceRoot}/webservice",
      "sourceLanguages": ["rust"],
      "cargo": {
        "args": ["build", "--bin=webservice"]
      }
    }
  ]
}
```

Make sure to enable the `"debug.allowBreakpointsEverywhere": true` setting in order to be able to put breakpoints.
Also make sure to have `"lldb.showDisassembly": "auto"` if you want to not have to debug using annotated ASM code.

You should now be able to build & debug your rust program automatically from VScode :)

Unit tests and integration tests can also be debugged in a similar approach:

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug Test WebService",
      "cargo": {
        "args": ["test", "--no-run"],
        "filter": {
          "name": "webservice",
          "kind": "lib"
        }
      },
      "args": ["${selectedText}"],
      "cwd": "${workspaceRoot}",
      "console": "internalConsole",
      "sourceLanguages": ["rust"]
    },
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug Test WebService Integration",
      "cargo": {
        "args": ["test", "--no-run"],
        "filter": {
          "name": "integration_test",
          "kind": "test"
        }
      },
      "args": ["${selectedText}"],
      "cwd": "${workspaceRoot}",
      "console": "internalConsole",
      "sourceLanguages": ["rust"]
    }
  ]
}
```

You'll notice that unit tests and integration tests will require separate launch configs,
given we require one specific artifact to match our specified filter, as we can only
run one artifact at any given time.

> Given that the dynamic content you can put in a Launch.json is limited to static info such
> as the current file or dir, I do not really see a way to make a launch config flexible enough
> to handle all this.
