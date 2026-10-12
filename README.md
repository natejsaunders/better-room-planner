# Room Planner

A lightweight room planner served locally by a small Rust standard-library web server. New plans start with a room drawing step: place wall points on the floor plan, finish the outline, then add furniture. Finished walls stay locked until you choose **Edit walls**; wall edits are rejected if they would leave furniture outside the room. Draw rectangular, L-shaped, U-shaped, or other polygon outlines; choose centimetres, metres, millimetres, inches, or feet; add standard furniture or named custom rectangles; and save plans on your computer. Dimensions are stored in centimetres.

Wall points and furniture use a 10 cm placement grid. Hold **Ctrl** while placing or moving a wall point to bypass the grid; it can still align horizontally or vertically with another point when it is within 10 cm. Drag wall points to adjust the outline while wall editing is enabled.

After finishing the room outline, click a furniture catalog item to place it near the room centre, or drag it onto the floor plan to see a full-size placement preview.

## Run

```sh
cargo run
```

Then open <http://127.0.0.1:3000>. To use a different port, pass it as the first argument, for example `cargo run -- 8080`.

Saved plans are stored as JSON files in a `plans/` folder under the current working directory. The browser also keeps an autosaved working copy locally.
