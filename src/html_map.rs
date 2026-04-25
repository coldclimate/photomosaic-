use std::collections::HashSet;
use std::path::Path;
use serde_json::json;

pub fn generate(tile_info: &str, mosaic_path: &Path, html_path: &Path) -> Result<(), String> {
    // Parse tile_info
    let lines: Vec<&str> = tile_info.lines().collect();
    if lines.is_empty() {
        return Err("Empty tile info".to_string());
    }

    // Parse grid dimensions from first line: "Grid dimension: WxH"
    let grid_line = lines[0];
    let (grid_w, grid_h) = parse_grid_dimensions(grid_line)?;

    // Parse grid data: one row per line, comma-separated filenames
    let mut grid: Vec<Vec<String>> = Vec::new();
    for row_idx in 1..lines.len() {
        let row_str = lines[row_idx];
        if row_str.is_empty() {
            continue;
        }
        let filenames: Vec<String> = row_str
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
        grid.push(filenames);
    }

    // Get mosaic image dimensions
    let mosaic_img = image::open(mosaic_path)
        .map_err(|e| format!("Failed to open mosaic image: {}", e))?;
    let img_width = mosaic_img.width();
    let img_height = mosaic_img.height();

    // Collect unique filenames
    let mut unique_tiles: HashSet<String> = HashSet::new();
    for row in &grid {
        for filename in row {
            unique_tiles.insert(filename.clone());
        }
    }
    let mut sorted_tiles: Vec<String> = unique_tiles.into_iter().collect();
    sorted_tiles.sort();

    // Build tile layout for JSON
    let mut tiles_json = Vec::new();
    for (row_idx, row) in grid.iter().enumerate() {
        for (col_idx, filename) in row.iter().enumerate() {
            tiles_json.push(json!({
                "row": row_idx,
                "col": col_idx,
                "file": filename
            }));
        }
    }

    // Create JSON data structure
    let data = json!({
        "grid_width": grid_w,
        "grid_height": grid_h,
        "destination_width": img_width,
        "destination_height": img_height,
        "tiles": tiles_json,
        "filenames": sorted_tiles
    });

    // Get mosaic image as file URL
    let mosaic_url = mosaic_path
        .canonicalize()
        .map_err(|e| format!("Failed to resolve mosaic image path: {}", e))
        .map(|p| format!("file://{}", p.display()))?;

    // Generate HTML
    let html = generate_html(&mosaic_url, &data)?;

    // Write HTML file
    std::fs::write(html_path, &html)
        .map_err(|e| format!("Failed to write HTML map: {}", e))?;

    // Write JSON data file (same directory as HTML, with .json extension)
    let json_path = html_path.with_extension("json");
    let json_str = serde_json::to_string(&data)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
    std::fs::write(&json_path, json_str)
        .map_err(|e| format!("Failed to write JSON data file: {}", e))?;

    Ok(())
}

fn parse_grid_dimensions(line: &str) -> Result<(u32, u32), String> {
    // Format: "Grid dimension: WxH"
    let parts: Vec<&str> = line.split(':').collect();
    if parts.len() < 2 {
        return Err("Invalid grid dimension format".to_string());
    }

    let dims_str = parts[1].trim();
    let dims: Vec<&str> = dims_str.split('x').collect();
    if dims.len() != 2 {
        return Err("Invalid grid dimension format".to_string());
    }

    let w = dims[0].trim().parse::<u32>()
        .map_err(|_| "Failed to parse grid width".to_string())?;
    let h = dims[1].trim().parse::<u32>()
        .map_err(|_| "Failed to parse grid height".to_string())?;

    Ok((w, h))
}

fn generate_html(mosaic_url: &str, data: &serde_json::Value) -> Result<String, String> {
    let _grid_w = data["grid_width"].as_u64().unwrap_or(0);
    let _grid_h = data["grid_height"].as_u64().unwrap_or(0);
    let _img_width = data["destination_width"].as_u64().unwrap_or(0);
    let _img_height = data["destination_height"].as_u64().unwrap_or(0);

    let mut html = String::new();

    html.push_str("<!DOCTYPE html>\n");
    html.push_str("<html>\n");
    html.push_str("<head>\n");
    html.push_str("<meta charset=\"UTF-8\">\n");
    html.push_str("<title>Source Image Finder</title>\n");
    html.push_str("<style>\n");
    html.push_str("* { margin: 0; padding: 0; box-sizing: border-box; }\n");
    html.push_str("body { display: flex; flex-direction: column; font-family: sans-serif; background: #f5f5f5; height: 100vh; }\n");
    html.push_str("#search-container { padding: 16px; background: #fff; border-bottom: 1px solid #ccc; }\n");
    html.push_str("input[type=\"text\"] { width: 100%; max-width: 400px; padding: 8px; font-size: 14px; border: 1px solid #ccc; border-radius: 4px; }\n");
    html.push_str("#autocomplete-list { position: absolute; background: #fff; border: 1px solid #ccc; border-top: none; list-style: none; max-height: 200px; overflow-y: auto; display: none; width: 100%; max-width: 400px; }\n");
    html.push_str("#autocomplete-list li { padding: 8px 12px; cursor: pointer; }\n");
    html.push_str("#autocomplete-list li:hover { background: #eee; }\n");
    html.push_str("#image-container { position: relative; flex: 1; overflow: auto; display: flex; align-items: center; justify-content: center; }\n");
    html.push_str("#mosaic-image { max-width: 100%; max-height: 100%; }\n");
    html.push_str("#grid-overlay { position: fixed; pointer-events: none; }\n");
    html.push_str(".grid-cell { position: absolute; border: 1px solid rgba(0, 0, 0, 0.1); box-sizing: border-box; }\n");
    html.push_str(".grid-cell.highlight { stroke: #ff0000 !important; stroke-width: 2 !important; }\n");
    html.push_str("</style>\n");
    html.push_str("</head>\n");
    html.push_str("<body>\n");

    html.push_str("<div id=\"search-container\">\n");
    html.push_str("  <input type=\"text\" id=\"search-input\" placeholder=\"Search for source image filename...\" autocomplete=\"off\">\n");
    html.push_str("  <ul id=\"autocomplete-list\"></ul>\n");
    html.push_str("</div>\n");

    html.push_str("<div id=\"image-container\">\n");
    html.push_str("  <img id=\"mosaic-image\" src=\"");
    html.push_str(mosaic_url);
    html.push_str("\" alt=\"Mosaic\">\n");
    html.push_str("  <svg id=\"grid-overlay\" style=\"position: fixed; pointer-events: none;\"></svg>\n");
    html.push_str("</div>\n");

    html.push_str("<script>\n");
    html.push_str("const data = ");
    html.push_str(&serde_json::to_string(&data).unwrap_or_default());
    html.push_str(";\n");
    html.push_str(
r#"
const searchInput = document.getElementById('search-input');
const autocompleteList = document.getElementById('autocomplete-list');
const mosaicImage = document.getElementById('mosaic-image');
const gridOverlay = document.getElementById('grid-overlay');
const imageContainer = document.getElementById('image-container');

let cellMap = {};
let imageRect = null;

function createGridCells() {
    // Build a map of filename -> array of {row, col}
    const filenameMap = {};
    data.tiles.forEach(tile => {
        if (!filenameMap[tile.file]) {
            filenameMap[tile.file] = [];
        }
        filenameMap[tile.file].push({row: tile.row, col: tile.col});
    });
    cellMap = filenameMap;
}

function getImageRect() {
    if (!imageRect) {
        imageRect = mosaicImage.getBoundingClientRect();
    }
    return imageRect;
}

function drawGrid() {
    gridOverlay.innerHTML = '';
    const rect = getImageRect();

    // Position SVG at the image's screen position
    gridOverlay.style.top = rect.top + 'px';
    gridOverlay.style.left = rect.left + 'px';
    gridOverlay.setAttribute('width', rect.width);
    gridOverlay.setAttribute('height', rect.height);

    const cellWidth = rect.width / data.grid_width;
    const cellHeight = rect.height / data.grid_height;

    for (let row = 0; row < data.grid_height; row++) {
        for (let col = 0; col < data.grid_width; col++) {
            const x = col * cellWidth;
            const y = row * cellHeight;

            const rect_elem = document.createElementNS('http://www.w3.org/2000/svg', 'rect');
            rect_elem.setAttribute('x', x);
            rect_elem.setAttribute('y', y);
            rect_elem.setAttribute('width', cellWidth);
            rect_elem.setAttribute('height', cellHeight);
            rect_elem.setAttribute('fill', 'none');
            rect_elem.setAttribute('stroke', 'rgba(0, 0, 0, 0.1)');
            rect_elem.setAttribute('stroke-width', '1');
            rect_elem.setAttribute('class', `grid-cell row-${row} col-${col}`);
            gridOverlay.appendChild(rect_elem);
        }
    }
}

function updateAutocomplete(value) {
    autocompleteList.innerHTML = '';
    if (!value.trim()) {
        autocompleteList.style.display = 'none';
        return;
    }

    const filtered = data.filenames.filter(f =>
        f.toLowerCase().includes(value.toLowerCase())
    );

    if (filtered.length === 0) {
        autocompleteList.style.display = 'none';
        return;
    }

    filtered.slice(0, 10).forEach(filename => {
        const li = document.createElement('li');
        li.textContent = filename;
        li.addEventListener('click', () => {
            searchInput.value = filename;
            autocompleteList.style.display = 'none';
            highlightCells(filename);
        });
        autocompleteList.appendChild(li);
    });

    autocompleteList.style.display = 'block';
}

function highlightCells(filename) {
    // Clear all highlights
    document.querySelectorAll('rect').forEach(rect => {
        rect.setAttribute('stroke', 'rgba(0, 0, 0, 0.1)');
        rect.setAttribute('stroke-width', '1');
    });

    // Highlight matching cells
    if (cellMap[filename]) {
        cellMap[filename].forEach(pos => {
            const selector = `.row-${pos.row}.col-${pos.col}`;
            const cells = document.querySelectorAll(selector);
            cells.forEach(cell => {
                cell.setAttribute('stroke', '#ff0000');
                cell.setAttribute('stroke-width', '2');
            });
        });
    }
}

searchInput.addEventListener('input', (e) => {
    updateAutocomplete(e.target.value);
});

searchInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
        const value = searchInput.value.trim();
        if (value && data.filenames.includes(value)) {
            highlightCells(value);
            autocompleteList.style.display = 'none';
        }
    }
});

// Redraw grid on image load, window resize, and scroll
function redrawGrid() {
    imageRect = null;
    drawGrid();
}

mosaicImage.addEventListener('load', redrawGrid);
window.addEventListener('resize', redrawGrid);
imageContainer.addEventListener('scroll', redrawGrid);

// Initialize
createGridCells();
if (mosaicImage.complete) {
    drawGrid();
} else {
    mosaicImage.addEventListener('load', drawGrid);
}
"#
    );
    html.push_str("</script>\n");

    html.push_str("</body>\n");
    html.push_str("</html>\n");

    Ok(html)
}
