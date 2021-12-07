use console::ransid::Color;
use failure::Error;
use std::convert::{TryFrom, TryInto};
use std::error::Error as StdError;
use std::fmt::Write;
use std::fs::{self, File};
use std::io::{Read, Write as OtherWrite};
use std::num::ParseIntError;
use std::path::{Path, PathBuf};
use toml;
use xdg::BaseDirectories;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Hex(String);

impl TryInto<Color> for Hex {
    type Error = Box<dyn StdError>;

    fn try_into(self) -> Result<Color, Self::Error> {
        let hex = self.0.trim_start_matches("#");
        let hex = decode_hex(hex)?;

        Ok(Color::TrueColor(hex[0], hex[1], hex[2]))
    }
}

impl From<Color> for Hex {
    fn from(value: Color) -> Self {
        fn encode_rgb(r: u8, g: u8, b: u8) -> String {
            let mut hex = String::new();
            write!(hex, "#{:02x}{:02x}{:02x}", r, g, b).unwrap();
            hex
        }

        Hex(match value {
            Color::TrueColor(r, g, b) => encode_rgb(r, g, b),
            Color::Ansi(value) => match value {
                0 => encode_rgb(0x00, 0x00, 0x00),
                1 => encode_rgb(0x80, 0x00, 0x00),
                2 => encode_rgb(0x00, 0x80, 0x00),
                3 => encode_rgb(0x80, 0x80, 0x00),
                4 => encode_rgb(0x00, 0x00, 0x80),
                5 => encode_rgb(0x80, 0x00, 0x80),
                6 => encode_rgb(0x00, 0x80, 0x80),
                7 => encode_rgb(0xc0, 0xc0, 0xc0),
                8 => encode_rgb(0x80, 0x80, 0x80),
                9 => encode_rgb(0xff, 0x00, 0x00),
                10 => encode_rgb(0x00, 0xff, 0x00),
                11 => encode_rgb(0xff, 0xff, 0x00),
                12 => encode_rgb(0x00, 0x00, 0xff),
                13 => encode_rgb(0xff, 0x00, 0xff),
                14 => encode_rgb(0x00, 0xff, 0xff),
                15 => encode_rgb(0xff, 0xff, 0xff),
                16..=231 => {
                    let convert = |value: u8| -> u8 {
                        match value {
                            0 => 0,
                            _ => value * 0x28 + 0x28,
                        }
                    };

                    let r = convert((value - 16) / 36 % 6);
                    let g = convert((value - 16) / 6 % 6);
                    let b = convert((value - 16) % 6);
                    encode_rgb(r, g, b)
                }
                232..=255 => {
                    let gray = (value - 232) * 10 + 8;
                    encode_rgb(gray, gray, gray)
                }
                _ => encode_rgb(0, 0, 0),
            },
        })
    }
}

pub fn decode_hex(s: &str) -> Result<Vec<u8>, ParseIntError> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect()
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub font: Option<String>,
    pub font_bold: Option<String>,
    pub background_color: Option<Hex>,
    pub save_scale: Option<bool>,
    pub columns: Option<u32>,
    pub rows: Option<u32>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            font: None,
            font_bold: None,
            background_color: None,
            save_scale: Some(true),
            columns: None,
            rows: None,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, Error> {
        let xdg = BaseDirectories::with_prefix("orbterm")?;

        if let Some(path) = xdg.find_config_file("config") {
            Config::read(&path)
        } else {
            let path = xdg.place_config_file("config")?;
            let config = Config::default();
            config.write(&path)?;
            Ok(config)
        }
    }

    pub fn get_config_path(file_name: &str) -> Result<PathBuf, Error> {
        let xdg = BaseDirectories::with_prefix("orbterm")?;
        Ok(xdg.place_config_file(file_name)?)
    }

    pub fn read<P: AsRef<Path>>(path: &P) -> Result<Self, Error> {
        let mut file = File::open(path)?;
        let mut contents = Vec::new();

        file.read_to_end(&mut contents)?;

        toml::from_slice(&contents).map_err(Error::from)
    }

    pub fn write<P: AsRef<Path>>(&self, path: &P) -> Result<(), Error> {
        let contents = toml::to_string_pretty(&self)?;
        let mut file = File::create(path)?;
        file.write_all(contents.as_bytes()).map_err(Error::from)
    }

    pub fn get_initial_scale(&self, display_height: u32) -> Result<f32, Error> {
        let config_path = Config::get_config_path("scale")?;

        let scale = (display_height / 1600) + 1;

        if self.save_scale.is_some() && self.save_scale.unwrap() {
            if config_path.exists() {
                let mut file = File::open(&config_path)?;
                let mut contents = String::new();
                file.read_to_string(&mut contents)?;
                let scale = contents.parse::<f32>()?;
                return Ok(scale);
            } else {
                Config::set_initial_scale(scale as f32)?;
            }
        }

        Ok(scale as f32)
    }

    pub fn set_initial_scale(scale: f32) -> Result<(), Error> {
        let config_path = Self::get_config_path("scale")?;
        fs::write(&config_path, scale.to_string())?;
        Ok(())
    }
}
