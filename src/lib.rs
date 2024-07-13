use ratatui::{buffer::Buffer, layout::Rect, style::Color, widgets::Widget};
use std::io::{BufRead, ErrorKind, Result, Seek};

/// An image pixel color, represented as RGBA
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Pixel {
    /// Red channel
    pub r: u8,
    /// Green channel
    pub g: u8,
    /// Blue channel
    pub b: u8,
    /// Alpha channel
    pub a: u8,
}

/// A single frame image, represented as a 2D array of RGBA pixels
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Image {
    pixels: Vec<Pixel>,
    width: usize,
    height: usize,
}

/// Fit mode for rendering an [`ImageView`]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Fit {
    /// The image will be zoomed to fit the render area, but dimension ratio is preserved.
    /// If the render area does not match the dimension ratio of the image, the image
    /// will be centered, and unused portions of the render area will be
    /// [`Color::Reset`].
    #[default]
    Zoom,
    /// The image will be streched to fit the entire render area. The image will be
    /// distorted if the render area does not match the dimension ratio of the [`ImageView`].
    Stretch,
}

/// Coordinates of a region of an image.
///
/// An [`ImageView`] may only reference a smaller section of the original
/// image. An instance of this type is used to designate the location of
/// that sub-section within the original image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Region {
    /// The X-coordinate (horizontal) of the top-left pixel of the region
    pub x: usize,
    /// The Y-coordinate (vertical) of the top-left pixel of the region
    pub y: usize,
    /// The width of the region, in pixels
    pub width: usize,
    /// The height of the region, in pixels
    pub height: usize,
}

/// Background color for rendering an [`ImageView`].
///
/// Background color is only relevant when an image contains pixels with
/// an 'alpha' component. Pixels with an alpha component are blended with
/// this background color before rendering, according to the alpha value.
/// An alpha value of 0 means the rendered pixel will be exactly the
/// background color, and an alpha value of 255 will be the unmodified
/// pixel color. Values in-between result in blending.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct BgColor {
    /// Red channel
    pub r: u8,
    /// Green channel
    pub g: u8,
    /// Blue channel
    pub b: u8,
}

/// A renderable view of an image.
///
/// An [`ImageView`] may represent only a specific region of the original
/// [`Image`], and it specifies how to fit the render area and how to handle
/// the alpha channel, if any. See also [`Region`], [`Fit`], and [`BgColor`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImageView<'a> {
    image: &'a Image,
    fit: Fit,
    region: Region,
    bg: BgColor,
}

/// An iterator over the pixels of an [`ImageView`].
///
/// Pixels are ordered starting from the top-left pixel, left to right,
/// then top to bottom (i.e. row by row).
#[derive(Debug, Clone)]
pub struct ViewPixels<'a> {
    pixels: &'a [Pixel],
    region: Region,
    real_width: usize,
    x: usize,
    y: usize,
}

fn u16_to_u8(value: u16) -> u8 {
    (value >> 8) as u8
}

fn f32_to_u8(value: f32) -> u8 {
    let value = value * 255.0;
    if value < 0.0 {
        0
    } else if value > 255.0 {
        255
    } else {
        value as u8
    }
}

impl Image {
    fn new_gray8(im: image::GrayImage) -> Self {
        let (width, height) = im.dimensions();
        let width = width as usize;
        let height = height as usize;
        let mut pixels = Vec::with_capacity(width * height);
        for pix in im.pixels() {
            pixels.push(Pixel {
                r: pix.0[0],
                g: pix.0[0],
                b: pix.0[0],
                a: 255,
            });
        }
        Self {
            pixels,
            width,
            height,
        }
    }

    fn new_grayalpha8(im: image::GrayAlphaImage) -> Self {
        let (width, height) = im.dimensions();
        let width = width as usize;
        let height = height as usize;
        let mut pixels = Vec::with_capacity(width * height);
        for pix in im.pixels() {
            pixels.push(Pixel {
                r: pix.0[0],
                g: pix.0[0],
                b: pix.0[0],
                a: pix.0[1],
            });
        }
        Self {
            pixels,
            width,
            height,
        }
    }

    fn new_rgb8(im: image::RgbImage) -> Self {
        let (width, height) = im.dimensions();
        let width = width as usize;
        let height = height as usize;
        let mut pixels = Vec::with_capacity(width * height);
        for pix in im.pixels() {
            pixels.push(Pixel {
                r: pix.0[0],
                g: pix.0[1],
                b: pix.0[2],
                a: 255,
            });
        }
        Self {
            pixels,
            width,
            height,
        }
    }

    fn new_rgba8(im: image::RgbaImage) -> Self {
        let (width, height) = im.dimensions();
        let width = width as usize;
        let height = height as usize;
        let mut pixels = Vec::with_capacity(width * height);
        for pix in im.pixels() {
            pixels.push(Pixel {
                r: pix.0[0],
                g: pix.0[1],
                b: pix.0[2],
                a: pix.0[3],
            });
        }
        Self {
            pixels,
            width,
            height,
        }
    }

    fn new_gray16(im: image::ImageBuffer<image::Luma<u16>, Vec<u16>>) -> Self {
        let (width, height) = im.dimensions();
        let width = width as usize;
        let height = height as usize;
        let mut pixels = Vec::with_capacity(width * height);
        for pix in im.pixels() {
            let val = u16_to_u8(pix.0[0]);
            pixels.push(Pixel {
                r: val,
                g: val,
                b: val,
                a: 255,
            });
        }
        Self {
            pixels,
            width,
            height,
        }
    }

    fn new_grayalpha16(im: image::ImageBuffer<image::LumaA<u16>, Vec<u16>>) -> Self {
        let (width, height) = im.dimensions();
        let width = width as usize;
        let height = height as usize;
        let mut pixels = Vec::with_capacity(width * height);
        for pix in im.pixels() {
            let val = u16_to_u8(pix.0[0]);
            pixels.push(Pixel {
                r: val,
                g: val,
                b: val,
                a: u16_to_u8(pix.0[1]),
            });
        }
        Self {
            pixels,
            width,
            height,
        }
    }

    fn new_rgb16(im: image::ImageBuffer<image::Rgb<u16>, Vec<u16>>) -> Self {
        let (width, height) = im.dimensions();
        let width = width as usize;
        let height = height as usize;
        let mut pixels = Vec::with_capacity(width * height);
        for pix in im.pixels() {
            pixels.push(Pixel {
                r: u16_to_u8(pix.0[0]),
                g: u16_to_u8(pix.0[1]),
                b: u16_to_u8(pix.0[2]),
                a: 255,
            });
        }
        Self {
            pixels,
            width,
            height,
        }
    }

    fn new_rgba16(im: image::ImageBuffer<image::Rgba<u16>, Vec<u16>>) -> Self {
        let (width, height) = im.dimensions();
        let width = width as usize;
        let height = height as usize;
        let mut pixels = Vec::with_capacity(width * height);
        for pix in im.pixels() {
            pixels.push(Pixel {
                r: u16_to_u8(pix.0[0]),
                g: u16_to_u8(pix.0[1]),
                b: u16_to_u8(pix.0[2]),
                a: u16_to_u8(pix.0[3]),
            });
        }
        Self {
            pixels,
            width,
            height,
        }
    }

    fn new_rgb32f(im: image::Rgb32FImage) -> Self {
        let (width, height) = im.dimensions();
        let width = width as usize;
        let height = height as usize;
        let mut pixels = Vec::with_capacity(width * height);
        for pix in im.pixels() {
            pixels.push(Pixel {
                r: f32_to_u8(pix.0[0]),
                g: f32_to_u8(pix.0[1]),
                b: f32_to_u8(pix.0[2]),
                a: 255,
            });
        }
        Self {
            pixels,
            width,
            height,
        }
    }

    fn new_rgba32f(im: image::Rgba32FImage) -> Self {
        let (width, height) = im.dimensions();
        let width = width as usize;
        let height = height as usize;
        let mut pixels = Vec::with_capacity(width * height);
        for pix in im.pixels() {
            pixels.push(Pixel {
                r: f32_to_u8(pix.0[0]),
                g: f32_to_u8(pix.0[1]),
                b: f32_to_u8(pix.0[2]),
                a: f32_to_u8(pix.0[3]),
            });
        }
        Self {
            pixels,
            width,
            height,
        }
    }

    fn from_image(im: image::DynamicImage) -> Self {
        match im {
            image::DynamicImage::ImageLuma8(im) => Self::new_gray8(im),
            image::DynamicImage::ImageLumaA8(im) => Self::new_grayalpha8(im),
            image::DynamicImage::ImageRgb8(im) => Self::new_rgb8(im),
            image::DynamicImage::ImageRgba8(im) => Self::new_rgba8(im),
            image::DynamicImage::ImageLuma16(im) => Self::new_gray16(im),
            image::DynamicImage::ImageLumaA16(im) => Self::new_grayalpha16(im),
            image::DynamicImage::ImageRgb16(im) => Self::new_rgb16(im),
            image::DynamicImage::ImageRgba16(im) => Self::new_rgba16(im),
            image::DynamicImage::ImageRgb32F(im) => Self::new_rgb32f(im),
            image::DynamicImage::ImageRgba32F(im) => Self::new_rgba32f(im),
            _ => {
                todo!()
            }
        }
    }

    fn from_reader<R: BufRead + Seek>(r: image::io::Reader<R>) -> Result<Self> {
        use image::error::ImageError;

        match r.decode() {
            Ok(im) => Ok(Self::from_image(im)),
            Err(ImageError::Decoding(_)) => Err(ErrorKind::InvalidData.into()),
            Err(ImageError::Encoding(_)) => Err(ErrorKind::InvalidData.into()),
            Err(ImageError::Parameter(_)) => Err(ErrorKind::InvalidInput.into()),
            Err(ImageError::Limits(_)) => Err(ErrorKind::InvalidData.into()),
            Err(ImageError::Unsupported(_)) => Err(ErrorKind::Unsupported.into()),
            Err(ImageError::IoError(e)) => Err(e),
        }
    }

    /// Loads an image from a type implementing [`BufRead`] and [`Seek`].
    /// The image format is automatically detected from the content.
    pub fn load<R: BufRead + Seek>(im: R) -> Result<Self> {
        Self::from_reader(image::io::Reader::new(im).with_guessed_format()?)
    }

    /// Opens an image file from disk. The file format is automatially detected
    /// based on the path and the content.
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        Self::from_reader(image::io::Reader::open(path)?)
    }

    /// Creates a new image with the provided size.
    ///
    /// All pixels will be initialized to be solid black.
    ///
    /// This constructor exists to allow generating an image from scratch. Use
    /// the [`pixels_mut`](Image::pixels_mut) and [`pixel_mut`](Image::pixel_mut)
    /// methods to paint pixels.
    pub fn with_size(width: usize, height: usize) -> Self {
        Self {
            pixels: vec![
                Pixel {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 255
                };
                width * height
            ],
            width,
            height,
        }
    }

    /// The width of the image, in pixels
    pub fn width(&self) -> usize {
        self.width
    }

    /// The height of the image, in pixels
    pub fn height(&self) -> usize {
        self.height
    }

    /// The width of the image, in terminal cells
    pub fn cell_width(&self) -> usize {
        self.width
    }

    /// The height of the image, in terminal cells, rounded up
    pub fn cell_height(&self) -> usize {
        self.height / 2 + self.height % 2
    }

    /// The pixels of the image.
    ///
    /// Pixels are ordered starting from the top-left pixel, left to right,
    /// then top to bottom (i.e. row by row).
    pub fn pixels(&self) -> &[Pixel] {
        &self.pixels[..]
    }

    /// The pixels of the image.
    ///
    /// Pixels are ordered starting from the top-left pixel, left to right,
    /// then top to bottom (i.e. row by row).
    pub fn pixels_mut(&mut self) -> &mut [Pixel] {
        &mut self.pixels[..]
    }

    /// Gets the pixel value at given pixel coordinates. [`None`](std::option::Option)
    /// is returned if the coordinates are out of bounds.
    pub fn pixel(&self, x: usize, y: usize) -> Option<&Pixel> {
        if x >= self.width || y >= self.height {
            None
        } else {
            Some(&self.pixels[(y * self.width) + x])
        }
    }

    /// Gets the pixel value at given pixel coordinates. [`None`](std::option::Option)
    /// is returned if the coordinates are out of bounds.
    pub fn pixel_mut(&mut self, x: usize, y: usize) -> Option<&mut Pixel> {
        if x >= self.width || y >= self.height {
            None
        } else {
            Some(&mut self.pixels[(y * self.width) + x])
        }
    }

    /// Returns an [`ImageView`] of the entire image.
    ///
    /// The returned [`ImageView`] defaults to [`Fit::Zoom`] and black background
    /// color (`#000000`).
    pub fn view(&self) -> ImageView<'_> {
        ImageView::new(self)
    }
}

impl Region {
    /// The X-coordinate (horizontal) of the top-left pixel in terms of terminal cells
    pub fn cell_x(&self) -> usize {
        self.x
    }

    /// The Y-coordinate (vertical) of the top-left pixel in terms of terminal cells, rounded up
    pub fn cell_y(&self) -> usize {
        self.y / 2 + self.y % 2
    }

    /// The width of the region, in terminal cells
    pub fn cell_width(&self) -> usize {
        self.width
    }

    /// The height of the region, in terminal cells, rounded up
    pub fn cell_height(&self) -> usize {
        self.height / 2 + self.height % 2
    }
}

fn apply_alpha(val: u8, bg: u8, alpha: u8) -> u8 {
    (((val as u16 * alpha as u16) + (bg as u16 * (255 - alpha) as u16)) / 255) as u8
}

impl Pixel {
    /// Converts a pixel to a [`Color`] value by blending with the provided background
    /// color based on the alpha channel when needed.
    pub fn on(&self, bg: BgColor) -> Color {
        Color::Rgb(
            apply_alpha(self.r, bg.r, self.a),
            apply_alpha(self.g, bg.g, self.a),
            apply_alpha(self.b, bg.b, self.a),
        )
    }
}

impl<'a> Iterator for ViewPixels<'a> {
    type Item = &'a Pixel;

    fn next(&mut self) -> Option<Self::Item> {
        if self.x == usize::MAX {
            return None;
        }

        let x = self.x;
        let y = self.y;
        self.x += 1;
        if self.x >= self.region.x + self.region.width {
            self.x = self.region.x;
            self.y += 1;
            if self.y >= self.region.y + self.region.height {
                self.x = usize::MAX;
                self.y = usize::MAX;
            }
        }

        Some(&self.pixels[(y * self.real_width) + x])
    }
}

impl From<Rect> for Region {
    fn from(area: Rect) -> Self {
        Self {
            x: area.x as usize,
            y: area.y as usize * 2,
            width: area.width as usize,
            height: area.height as usize * 2,
        }
    }
}

impl From<BgColor> for Color {
    fn from(bg: BgColor) -> Self {
        Color::Rgb(bg.r, bg.g, bg.b)
    }
}

impl<'a> ImageView<'a> {
    /// Returns an [`ImageView`] of the entire image.
    ///
    /// The returned [`ImageView`] defaults to [`Fit::Zoom`] and black background
    /// color (`#000000`).
    pub fn new(image: &'a Image) -> Self {
        let width = image.width;
        let height = image.height;
        Self {
            image,
            fit: Fit::Zoom,
            region: Region {
                x: 0,
                y: 0,
                width,
                height,
            },
            bg: BgColor::default(),
        }
    }

    /// Factory pattern setter for the [`Fit`] mode of the view
    pub fn with_fit(mut self, fit: Fit) -> Self {
        self.set_fit(fit);
        self
    }

    /// Factory pattern setter for the [`Region`] of the view
    pub fn with_region(mut self, region: Region) -> Self {
        self.set_region(region);
        self
    }

    /// Factory pattern setter for the background color of the view
    pub fn with_bg_color(mut self, color: BgColor) -> Self {
        self.set_bg_color(color);
        self
    }

    /// Setter for the [`Fit`] mode of the view
    pub fn set_fit(&mut self, fit: Fit) {
        self.fit = fit;
    }

    /// Setter for the [`Region`] of the view
    pub fn set_region(&mut self, region: Region) {
        let Region {
            mut x,
            mut y,
            mut width,
            mut height,
        } = region;
        if x > self.image.width || y > self.image.height {
            x = 0;
            y = 0;
            width = 0;
            height = 0;
        } else {
            if x + width > self.image.width {
                width = self.image.width - x;
            }
            if y + height > self.image.height {
                height = self.image.height - y;
            }
        }
        self.region = Region {
            x,
            y,
            width,
            height,
        };
    }

    /// Setter for the background color of the view
    pub fn set_bg_color(&mut self, color: BgColor) {
        self.bg = color;
    }

    /// Gets the original image
    pub fn image(&self) -> &'a Image {
        self.image
    }

    /// Gets the current [`Fit`] mode of the view
    pub fn fit(&self) -> Fit {
        self.fit
    }

    /// Gets the current [`Region`] of the view
    pub fn region(&self) -> &Region {
        &self.region
    }

    /// Returns an iterator over the pixels of the view according to its [`Region`]
    pub fn pixels(&self) -> ViewPixels<'a> {
        ViewPixels {
            pixels: self.image.pixels(),
            region: self.region,
            real_width: self.image.width,
            x: self.region.x,
            y: self.region.y,
        }
    }

    /// Gets the pixel value at given pixel coordinates. [`None`](std::option::Option)
    /// is returned if the coordinates are out of bounds.
    pub fn pixel(&self, x: usize, y: usize) -> Option<Pixel> {
        if x >= self.region.width || y >= self.region.height {
            None
        } else {
            self.image
                .pixel(x + self.region.x, y + self.region.y)
                .copied()
        }
    }
}

const PIXEL_CHAR: char = '▀';

impl<'a> Widget for ImageView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width as usize == self.region.width
            && area.height as usize * 2 == self.region.height
        {
            for x in 0..area.width {
                for y in 0..area.height {
                    let pix_x = x as usize;
                    let pix_y1 = y as usize * 2;
                    let pix_y2 = pix_y1 + 1;
                    let pix1 = self.pixel(pix_x, pix_y1).unwrap_or_default().on(self.bg);
                    let pix2 = self.pixel(pix_x, pix_y2).unwrap_or_default().on(self.bg);
                    buf.get_mut(x, y)
                        .set_char(PIXEL_CHAR)
                        .set_fg(pix1)
                        .set_bg(pix2);
                }
            }
        } else {
            let mut zoom_x = area.width as f32 / self.region.width as f32;
            let mut zoom_y = area.height as f32 * 2.0 / self.region.height as f32;
            let mut x_pos = 0u16;
            let mut y_pos = 0u16;
            if let Fit::Zoom = self.fit {
                if zoom_x < zoom_y {
                    y_pos = (((area.height as usize * 2)
                        - (self.region.height as f32 * zoom_x) as usize)
                        / 4) as u16;
                    zoom_y = zoom_x;
                } else {
                    x_pos = ((area.width as usize - (self.region.width as f32 * zoom_y) as usize)
                        / 2) as u16;
                    zoom_x = zoom_y;
                }
            }

            for x in 0..area.width {
                for y in 0..area.height {
                    if x < x_pos || y < y_pos {
                        buf.get_mut(x, y).set_char(' ').set_bg(Color::Reset);
                        continue;
                    }
                    let pix_x = ((x - x_pos) as f32 / zoom_x) as usize;
                    let y1 = (y - y_pos) as usize * 2;
                    let y2 = y1 + 1;
                    let pix_y1 = (y1 as f32 / zoom_y) as usize;
                    let pix_y2 = (y2 as f32 / zoom_y) as usize;
                    let pix1 = self.pixel(pix_x, pix_y1);
                    let pix2 = self.pixel(pix_x, pix_y2);
                    if pix1.is_none() && pix2.is_none() {
                        buf.get_mut(x, y).set_char(' ').set_bg(Color::Reset);
                        continue;
                    }
                    let pix1 = match pix1 {
                        None => Color::Reset,
                        Some(pix) => pix.on(self.bg),
                    };
                    let pix2 = match pix2 {
                        None => Color::Reset,
                        Some(pix) => pix.on(self.bg),
                    };
                    buf.get_mut(x, y)
                        .set_char(PIXEL_CHAR)
                        .set_fg(pix1)
                        .set_bg(pix2);
                }
            }
        }
    }
}
