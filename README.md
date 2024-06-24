# Task Warrior Web UI focusing on Keyboard navigation

This is a straightforward, but mostly working task warrior web ui with strong focus on keyboard navigation.
It's completely local. No intention to have any kind of online interactions.

## Stack
* Rust
* axum
* tera
* TailwindCSS
* HTMX
* hotkeys
* rollup
* Task Warrior (obviously :) )

Still work in progress. But in the current stage it is pretty usable


# Building and Running

1. Clone the latest version from GitHub.
2. `cd frontend`
3. `npm install`
4. `cd ..`
5. `cargo run --release`

That should be it! Now you have the server running at `localhost:3000` accessible by your browser.

## Customizing the port
By default, the program will use 3000 as port,
you can customize through `.env` file or enviornment variable, check `env.example`

variable name: `TWK_SERVER_PORT`

```shell
TWK_SERVER_PORT=9070 cargo run --release
```


# Using the app

You can use Mouse or Keyboard to navigate.

![Top bar](./screenshots/top_bars.png)

* All the keyboard mnemonics are underlined. 
* The `Cmd Bar` needs to be focused (`Ctrl + Shift + K`) for the keyboard shortcuts to work

## Project and Tag selection
Keyboard shortcut is `t`

For selecting tag, once you enter tag selection mode, the `tag bar` is visible,
tag mnemonics are displayed on the tags, in red boxes, typing the mnemonics will immediately set the tag/project

![Search bar](./screenshots/tag-search.png)

## Creating new task
Keyboard shortcut is `n`

Which should bring up the new task dialog box. It will use the current tags and project to create the task
![New task](./screenshots/new-task.png)

## Undo
Keyboard shortcut is `u`

This will bring up undo confirmation dialog
![Undo](./screenshots/undo.png)


# WIP warning

This is a work in progress application, many things will not work,
there will be errors, as no checks, and there may not be any error messages in case of error. 

- [ ] Usability improvements on long task list
- [x] Marking a task done with keyboard shortcut
- [x] Bug fix, not unmarking completed task
- [ ] Editing/Deleting/Starting tasks
- [ ] Following Context
- [ ] Error handling
- [ ] Add more tests
- [x] Which port to run
- [ ] Convert to desktop app using Tauri
