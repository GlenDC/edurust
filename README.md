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
      // "preLaunchTask": "build",
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
