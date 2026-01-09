# Image Module

The `Image` module provides image loading, processing, and saving capabilities.

## Overview

```stratum
let img = Image.open("photo.png")
let resized = img.resize(800, 600)
resized.save("thumbnail.png")
```

## Supported Formats

| Format | Read | Write |
|--------|------|-------|
| PNG    | Yes  | Yes   |
| JPEG   | Yes  | Yes   |
| GIF    | Yes  | Yes   |
| BMP    | Yes  | Yes   |
| WebP   | Yes  | Yes   |

## Static Methods

### Image.open(path: String) -> Image

Load an image from a file.

**Parameters:**
- `path` - Path to the image file

**Returns:** An `Image` object

**Example:**
```stratum
let img = Image.open("photo.jpg")
print(img.width())   // 1920
print(img.height())  // 1080
```

### Image.new(width: Int, height: Int, color?: String) -> Image

Create a new blank image.

**Parameters:**
- `width` - Image width in pixels
- `height` - Image height in pixels
- `color` - Optional fill color (default: white)

**Color Formats:**
- Hex: `"#RRGGBB"` or `"#RRGGBBAA"`
- Named: `"white"`, `"black"`, `"red"`, `"green"`, `"blue"`, `"yellow"`, `"cyan"`, `"magenta"`, `"transparent"`
- RGB List: `[r, g, b]` or `[r, g, b, a]`

**Example:**
```stratum
let blank = Image.new(100, 100)
let red = Image.new(200, 200, "red")
let custom = Image.new(50, 50, "#FF6600")
let rgba = Image.new(100, 100, [255, 128, 0, 128])
```

## Image Methods

### Properties

#### img.width() -> Int

Get the image width in pixels.

#### img.height() -> Int

Get the image height in pixels.

#### img.dimensions() -> List

Get width and height as a list `[width, height]`.

**Example:**
```stratum
let img = Image.open("photo.png")
let [w, h] = img.dimensions()
print("Size: " + w + "x" + h)
```

### Transformations

All transformation methods return a new Image, leaving the original unchanged.

#### img.resize(width: Int, height: Int) -> Image

Resize the image to the specified dimensions using high-quality Lanczos resampling.

**Example:**
```stratum
let thumb = img.resize(150, 150)
```

#### img.crop(x: Int, y: Int, width: Int, height: Int) -> Image

Crop a region from the image.

**Parameters:**
- `x`, `y` - Top-left corner of crop region
- `width`, `height` - Size of crop region

**Example:**
```stratum
let cropped = img.crop(100, 100, 400, 300)
```

#### img.rotate(degrees?: Int) -> Image

Rotate the image. Only supports 90, 180, or 270 degrees.

**Parameters:**
- `degrees` - Rotation angle (default: 90)

**Example:**
```stratum
let rotated90 = img.rotate()
let rotated180 = img.rotate(180)
let rotated270 = img.rotate(270)
```

#### img.flip_h() -> Image

Flip the image horizontally (mirror).

#### img.flip_v() -> Image

Flip the image vertically.

**Example:**
```stratum
let mirrored = img.flip_h()
let upside_down = img.flip_v()
```

### Color Operations

#### img.grayscale() -> Image

Convert the image to grayscale.

**Example:**
```stratum
let bw = img.grayscale()
```

#### img.invert() -> Image

Invert all colors in the image.

**Example:**
```stratum
let negative = img.invert()
```

#### img.brightness(value: Float) -> Image

Adjust image brightness.

**Parameters:**
- `value` - Brightness adjustment (-1.0 to 1.0)
  - Negative values darken the image
  - Positive values lighten the image

**Example:**
```stratum
let darker = img.brightness(-0.3)
let lighter = img.brightness(0.3)
```

#### img.contrast(value: Float) -> Image

Adjust image contrast.

**Parameters:**
- `value` - Contrast multiplier
  - 0.0 = flat gray
  - 1.0 = no change
  - >1.0 = increased contrast

**Example:**
```stratum
let low_contrast = img.contrast(0.5)
let high_contrast = img.contrast(1.5)
```

#### img.hue_rotate(degrees: Int) -> Image

Rotate the hue of all colors.

**Parameters:**
- `degrees` - Hue rotation in degrees (0-360)

**Example:**
```stratum
let shifted = img.hue_rotate(180)  // Opposite colors
```

#### img.saturate(value: Float) -> Image

Adjust color saturation.

**Parameters:**
- `value` - Saturation multiplier
  - 0.0 = grayscale
  - 1.0 = no change
  - >1.0 = more saturated

**Example:**
```stratum
let muted = img.saturate(0.5)
let vivid = img.saturate(1.5)
```

#### img.blur(sigma: Float) -> Image

Apply Gaussian blur to the image.

**Parameters:**
- `sigma` - Blur intensity (larger = more blur)

**Example:**
```stratum
let soft = img.blur(2.0)
let very_blurry = img.blur(10.0)
```

#### img.sharpen() -> Image

Sharpen the image using unsharp masking.

**Example:**
```stratum
let sharp = img.sharpen()
```

### I/O Operations

#### img.save(path: String) -> null

Save the image to a file. Format is determined by file extension.

**Parameters:**
- `path` - Output file path

**Example:**
```stratum
img.save("output.png")
img.resize(100, 100).save("thumb.jpg")
```

#### img.to_bytes(format?: String) -> List<Int>

Convert the image to bytes.

**Parameters:**
- `format` - Output format ("png", "jpg", "gif", "bmp", "webp"). Defaults to source format.

**Returns:** List of bytes (integers 0-255)

**Example:**
```stratum
let bytes = img.to_bytes("png")
File.write_bytes("output.png", bytes)
```

## Common Patterns

### Creating Thumbnails
```stratum
fx create_thumbnail(input_path, output_path, max_size) {
    let img = Image.open(input_path)
    let [w, h] = img.dimensions()

    // Calculate new dimensions maintaining aspect ratio
    let scale = max_size / Math.max(w, h)
    let new_w = (w * scale).floor()
    let new_h = (h * scale).floor()

    img.resize(new_w, new_h).save(output_path)
}

create_thumbnail("photo.jpg", "thumb.jpg", 200)
```

### Batch Processing
```stratum
let files = Dir.list("images/")
    .filter(|f| f.ends_with(".jpg"))

for file in files {
    let img = Image.open("images/" + file)
    img.grayscale()
       .contrast(1.2)
       .save("processed/" + file)
}
```

### Image Pipeline
```stratum
let result = Image.open("photo.jpg")
    |> .resize(800, 600)
    |> .brightness(0.1)
    |> .contrast(1.1)
    |> .sharpen()

result.save("enhanced.jpg")
```

### Color Correction
```stratum
let corrected = img
    .brightness(0.05)      // Slightly brighten
    .contrast(1.1)         // Add contrast
    .saturate(1.15)        // Boost colors
    .sharpen()             // Final sharpening
```
