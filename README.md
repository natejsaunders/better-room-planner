# Room Planner

A lightweight room planner served locally by a small Rust standard-library web server. Choose rectangular, L-shaped, U-shaped, or custom polygon room outlines; edit custom corner coordinates; choose centimetres, metres, millimetres, inches, or feet; add standard furniture or named custom rectangles at any size that fits; place and move furniture on a to-scale floor plan; and save plans on your computer. Dimensions are stored in centimetres.

## Run

```sh
cargo run
```

Then open <http://127.0.0.1:3000>. To use a different port, pass it as the first argument, for example `cargo run -- 8080`.

Saved plans are stored as JSON files in a `plans/` folder under the current working directory. The browser also keeps an autosaved working copy locally.
