# review_helper



https://github.com/user-attachments/assets/981f277a-9276-4fa8-928b-69bb7a5b0652



review_helper is a standalone code review tool that runs locally on the system and provides an graphical user interface.
The tool provides following functionality:

* Adding Git repositories and creating reviews
* Determine changes based on two commits or against a working copy
* File changes can be visualized using external tool like meld, vscode or what-ever-you-want-to-configure
* Mark changed files and add notes to them
* Apply various filter, sort mechanisms in different views
* Store review result based on text files (markdown, toml)

## Supported Platforms

* Linux
* Windows
* Macos

## Requirements to run

review_helper requires git to be installed!

## Getting review_helper

* download the release for you target system from GitHub
* build from source

## Build from source

First checkout the source from GitHub and execute the following commands:

```
~> cargo build --release
```

The binary can be found in the target folder.
In order to improve the font rendering set following enviroment variable `SLINT_BACKEND=winit-skia`.
