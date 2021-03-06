extern crate num;
extern crate image;
extern crate crossbeam;
extern crate num_cpus;

use std::str::FromStr;
use std::fs::File;

use num::Complex;
use num::Zero;

use image::ColorType;
use image::png::PNGEncoder;

fn calculate_escape_time(c: Complex<f64>, limit: u32) -> Option<u32> {
    let mut z = Complex::zero();

    for i in 0 .. limit {
        z = z * z + c;

        if z.norm() > 4.0 {
            return Some(i);
        }
    }

    None
}

fn parse_pair<T: FromStr>(s: &str, separator: char) -> Option<(T,T)> {
    match s.find(separator) {
        None => None,
        Some(index) => {
            match (T::from_str(&s[..index]), T::from_str(&s[index + 1..])) {
                (Ok(left), Ok(right)) => Some((left, right)),
                _ => None,
            }
        }
    }
}

#[test]
fn test_parse_pair() {
    assert_eq!(parse_pair::<i32>("",        ','), None);
    assert_eq!(parse_pair::<i32>("10,",     ','), None);
    assert_eq!(parse_pair::<i32>(",10",     ','), None);
    assert_eq!(parse_pair::<i32>("10,20",   ','), Some((10, 20)));
    assert_eq!(parse_pair::<i32>("10,20xy", ','), None);
    assert_eq!(parse_pair::<f64>("0.5x",    'x'), None);
    assert_eq!(parse_pair::<f64>("0.5x1.5", 'x'), Some((0.5, 1.5)));
}

fn parse_complex(s: &str) -> Option<Complex<f64>> {
    match parse_pair(s, ',') {
        Some((re, im)) => Some(Complex{re, im}),
        None => None,
    }
}

#[test]
fn test_parse_complex() {
    assert_eq!(parse_complex("1.25,-0.0625"),
               Some(Complex{ re: 1.25, im: -0.0625 }));

    assert_eq!(parse_complex(", -0.6123"), None);
}

fn pixel_to_point(bounds: (usize, usize),
                  pixel: (usize, usize),
                  upper_left: Complex<f64>,
                  lower_right: Complex<f64>) -> Complex<f64> {

    let (width, height) = (lower_right.re - upper_left.re,
                           upper_left.im - lower_right.im);

    Complex {
        re: upper_left.re + pixel.0 as f64 * width  / bounds.0 as f64,
        im: upper_left.im - pixel.1 as f64 * height / bounds.1 as f64,
    }
}

#[test]
fn test_pixel_to_point() {
    assert_eq!(pixel_to_point((100, 100), (25, 75),
                              Complex { re: -1.0, im: 1.0 },
                              Complex { re: 1.0, im: -1.0 }),
               Complex { re: -0.5, im: -0.5 });
}

fn render(pixels: &mut [u8],
          bounds: (usize, usize),
          upper_left: Complex<f64>,
          lower_right: Complex<f64>) {

    assert_eq!(pixels.len(), bounds.0 * bounds.1);

    for row in 0 .. bounds.1 {
        for column in 0 .. bounds.0 {
            let point = pixel_to_point(bounds,
                                       (column, row),
                                       upper_left,
                                       lower_right);

            pixels[row * bounds.0 + column] = match calculate_escape_time(point, 255) {
                None => 0,
                Some(count) => 255 - count as u8,
            }
        }
    }
}

fn write_image(filename: &str, pixels: &[u8], bounds: (usize, usize)) -> Result<(), std::io::Error> {
    let output = File::create(filename)?;

    let encoder = PNGEncoder::new(output);
    encoder.encode(pixels,
                   bounds.0 as u32, bounds.1 as u32,
                   ColorType::Gray(8))?;

    Ok(())
}

fn print_help_and_exit() -> ! {
    eprintln!("Usage: <file_to_be_saved> <bounds> <upper_left> <lower_right>");
    eprintln!("Example:\ncargo run --release -- fractal.png 1000x750 -1.20,0.35 -1,0.20");
    std::process::exit(1);
}

fn read_args() -> Vec<String> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 5 {
        print_help_and_exit();
    }

    args
}

fn save_fractal(args: Vec<String>) {
    let bounds = parse_pair(&args[2], 'x').expect("error parsing bounds");
    let upper_left = parse_complex(&args[3]).expect("error parsing upper left corner");
    let lower_right = parse_complex(&args[4]).expect("error parsing lower right corner");
    let mut pixels = vec![0; bounds.0 * bounds.1];
    let threads = num_cpus::get();
    let rows_per_band = bounds.1 / threads + 1;

    {
        let bands: Vec<&mut [u8]> = pixels.chunks_mut(rows_per_band * bounds.0).collect();
        crossbeam::scope(|spawner| {
            for (i, band) in bands.into_iter().enumerate() {
                let top = rows_per_band * i;
                let height = band.len() / bounds.0;
                let band_bounds = (bounds.0, height);
                let band_upper_left = pixel_to_point(bounds, (0, top),
                                                                   upper_left, lower_right);
                let band_lower_right = pixel_to_point(bounds, (bounds.0, top + height),
                                                                    upper_left, lower_right);

                spawner.spawn(move || {
                   render(band,band_bounds, band_upper_left, band_lower_right);
                });
            }
        });
    }

    write_image(&args[1], &pixels, bounds).expect("error writing PNG image");
}

fn main() {
    let args = read_args();
    save_fractal(args);
}