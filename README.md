# review_helper

review_helper is a standalone code review tool that runs locally on the system and provides an graphical user interface.
The tool provides following functionality:

* determine changes based on two commits or against a working copy
* file changes can be visualized using external tool like meld, vscode or what-ever-you-want-to-configure
* mark changed files and add notes to them
* apply various filter, sort mechanisms in different views
* store review result based on text files (markdown, toml)

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

Frist checkout the source from GitHub and execute the following commands:

```
~> cargo build --release
```

The binary can be found in the target folder.
In order to improve the font rendering set following enviroment variable `SLINT_BACKEND=winit-skia`.

## Motivations

In my job we did not use big source code management software(like GitLab) therefore we had no fancy tooling for code reviews.
So I did all the required steps of the review by hand which always takes a little more time. Thats why I created this project.

Further, I have always wanted to use and test the Slint UI Framework.
