use byteorder::{ByteOrder, LittleEndian};
use std::{collections::HashMap, fmt}; // 1.3.4
mod DataHelper;
use DataHelper::BitReader;
use DataHelper::BitState;

///
#[derive(Default)]
pub struct Gif {
    version: String,
    lsd: LogicalScreenDescriptor,
    global_table: Option<Vec<Color>>,
    frames: Vec<ParsedFrame>,
}
impl Gif {
    // fn example(&mut self) {}
}

#[derive(Default)]
pub(crate) struct LogicalScreenDescriptor {
    width: u16,
    height: u16,
    global_color_flag: bool,
    color_resolution: u8,
    sorted_flag: bool,
    global_color_size: u8,
    background_color_index: u8,
    pixel_aspect_ratio: u8,
}

#[derive(Default)]
struct ParsedFrame {
    gcd: GraphicsControlExtension,
    im: ImageDescriptor,
}

#[derive(Default)]
pub(crate) struct ImageDescriptor {
    left: u16,
    top: u16,
    width: u16,
    height: u16,
    local_color_table_flag: bool,
    interface_flag: bool,
    sort_flag: bool,
    local_color_table_size: u16,
}

#[derive(Default)]
pub(crate) struct GraphicsControlExtension {
    disposal_method: u8,
    user_input_flag: bool,
    transparent_color_flag: bool,
    delay_time: u16,
    transparent_color_index: u8,
}

#[derive(Clone)]
enum CodeTable {
    Color(Vec<u16>),
    Empty,
    Clear,
    End,
}

pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}
///
pub struct Decoder {
    pub offset: usize,
}

impl Decoder {
    pub fn decode(&mut self, file_path: &str) -> Result<(), GifError> {
        let contents = std::fs::read(file_path).expect("Something went wrong reading the file");

        let mut contents = contents.as_slice();
        {
            let mut signature: String = String::new();
            match String::from_utf8(contents[0..3].to_vec()) {
                Ok(parsed_signature) => {
                    signature = parsed_signature;
                }
                Err(err) => println!("Error 1: {}", err),
            }
            if signature != "GIF" {
                return Err(GifError::SignatureError);
            }
        }

        let mut gif = Gif::default();
        let mut version: String = String::new();
        match String::from_utf8(contents[3..6].to_vec()) {
            Ok(parsed_version) => {
                version = parsed_version;
            }
            Err(err) => println!("Error 2: {}", err),
        }
        gif.version = version;

        self.handle_logical_screen_descriptor(&mut gif, contents);

        self.offset = 13;
        // Global Color Table
        let length: usize = 3 * 2 << gif.lsd.global_color_size;
        let mut i: usize = self.offset;
        let mut global_color_vector: Vec<Color> = Vec::new();

        while i < self.offset + length {
            global_color_vector.push(Color {
                red: contents[i],
                green: contents[i + 1],
                blue: contents[i + 2],
                alpha: 255,
            });
            i = i + 3;
        }
        self.increment_offset(length);
        // End
        loop {
            let extension_introducer = contents[self.offset];
            if extension_introducer != 0x21 && extension_introducer == 0x3B {
                break;
            }
            println!("Offset: {}", self.offset);
            self.increment_offset(1);

            let label = contents[self.offset];
            self.increment_offset(1);
            match label {
                0xF9 => {
                    self.handle_graphic_control_extension(&mut gif, contents);
                }
                0x01 => {
                    self.handle_plain_text_extension(&mut gif, contents);
                }
                0xFF => {
                    self.handle_application_extension(&mut gif, contents);
                }
                0xFE => {
                    self.handle_comment_extension(&mut gif, contents);
                }
                _ => {}
            }
        }
        // Trailer
        println!("End of file.");
        return Ok(());
    }
    fn increment_offset(&mut self, amount: usize) {
        self.offset += amount;
    }
    fn shl_or(&mut self, val: u8, shift: usize, def: u8) -> u8 {
        [val << (shift & 7), def][((shift & !7) != 0) as usize]
    }
    fn shr_or(&mut self, val: u8, shift: usize, def: u8) -> u8 {
        [val >> (shift & 7), def][((shift & !7) != 0) as usize]
    }
    fn handle_logical_screen_descriptor(&mut self, gif: &mut Gif, contents: &[u8]) {
        gif.lsd.width = LittleEndian::read_u16(&contents[6..8]); // width
        gif.lsd.height = LittleEndian::read_u16(&contents[8..10]); // height

        let packed_field = contents[10];

        gif.lsd.global_color_flag = (packed_field & 0b1000_0000) != 0; // global_color_flag
        gif.lsd.color_resolution = (packed_field & 0b0111_0000) as u8; // color_resolution
        gif.lsd.sorted_flag = (packed_field & 0b0000_1000) != 0; // sorted_flag
        gif.lsd.global_color_size = (packed_field & 0b0000_0111) as u8; // global_color_size

        gif.lsd.background_color_index = contents[11]; // background_color_index
        gif.lsd.pixel_aspect_ratio = contents[12]; // pixel_aspect_ratio
    }
    fn handle_graphic_control_extension(&mut self, gif: &mut Gif, contents: &[u8]) {
        // Graphical Control Extension
        let byte_size = contents[self.offset];
        self.increment_offset(1);

        let packed_field = contents[self.offset];
        let disposal_method = (packed_field & 0b0001_1100) as u8;
        let user_input_flag = (packed_field & 0b0000_0010) != 0;
        let transparent_color_flag = (packed_field & 0b0000_0001) != 0;
        self.increment_offset(1);

        let delay_time = LittleEndian::read_u16(&contents[self.offset..self.offset + 2]);
        self.increment_offset(2);

        let transparent_color_index = contents[self.offset];
        println!("{}", transparent_color_index);
        self.increment_offset(1);

        let block_terminator = contents[self.offset]; // This must be 00
        self.increment_offset(1);
        // End

        // Image Descriptor
        let image_separator = contents[self.offset]; // This must be "2C" or 44
        self.increment_offset(1);

        let image_left = LittleEndian::read_u16(&contents[self.offset..self.offset + 2]);
        self.increment_offset(2);

        let image_top = LittleEndian::read_u16(&contents[self.offset..self.offset + 2]);
        self.increment_offset(2);

        let image_width = LittleEndian::read_u16(&contents[self.offset..self.offset + 2]);
        self.increment_offset(2);

        let image_height = LittleEndian::read_u16(&contents[self.offset..self.offset + 2]);
        self.increment_offset(2);

        let packed_field = contents[self.offset];
        let local_color_table_flag = (packed_field & 0b1000_0000) != 0;
        let interface_flag = (packed_field & 0b0100_0000) != 0;
        let sort_flag = (packed_field & 0b0010_0000) != 0;
        // let _ = (packed_field & 0b0001_1000) as u8; // Future use
        let local_color_table_size = (packed_field & 0b0000_0111) as u8;
        self.increment_offset(1);
        // End

        // Local Color Table
        if (local_color_table_flag) {
            let length: usize = 3 * 2 << local_color_table_size;
            let mut i: usize = self.offset;
            let mut local_color_vector: Vec<Color> = Vec::new();

            while i < self.offset + length {
                local_color_vector.push(Color {
                    red: contents[i],
                    green: contents[i + 1],
                    blue: contents[i + 2],
                    alpha: 255,
                });
                i = i + 3;
            }
            self.increment_offset(length);
        }
        // End

        // Image Data
        let lzw_minimum_code_size = contents[self.offset];
        self.increment_offset(1);

        // Data sub block section
        let mut data_sub_blocks_count = contents[self.offset];
        self.increment_offset(1);

        let mut index_stream: Vec<Option<u8>> = Vec::new();

        let mut code_units: Vec<CodeUnit> = Vec::new();

        let mut code_table: Vec<Option<u8>> = Vec::new();

        let mut code_stream: Vec<u8> = Vec::new();

        let clear_code = 2 << (lzw_minimum_code_size - 1);
        let eoi_code = clear_code + 1;

        let mut last_code = eoi_code;
        let mut size: usize = (lzw_minimum_code_size + 1).into();
        let mut grow_code: u8 = (2 << (lzw_minimum_code_size - 1)) - 1;
        // let mut previous_code: u8 = 0;
        
        let  mut is_initalized = false;

        let mut br = BitReader::new();
        loop {
            while data_sub_blocks_count > 0 {
                let content = contents[self.offset];
                br.pushByte(content);
                loop {
                    let code_start = br.get_state();
                    let code = br.readBits(size);
                    if (code == eoi_code) {
                        code_stream.push(code);
                        break;
                    } else if (code == clear_code) {
                        code_units.push(CodeUnit{stream: Vec::new(), table: Vec::new(), start: code_start});
                        code_stream = code_units[code_units.len() - 1].stream;
                        code_table = code_units[code_units.len() - 1].table;
                        for n in 0..eoi_code {
                            if n < clear_code {
                                code_table[n.into()] = Ok(n);
                            } else {
                                code_table[n.into()] = None;
                            }
                        }
                        last_code = eoi_code;
                        size = (lzw_minimum_code_size + 1).into();
                        grow_code = (2 << size - 1) - 1;
                        is_initalized = false;
                        
                    }  else if (!is_initalized) {
                        index_stream.push(...codeTable[code]);
                        is_initalized = true;
                    }
                    else {
                        let k = 0;
                        let prev_code = code_stream[code_stream.len() - 1];
                        if (code <= last_code) {
                            index_stream.push(code_table[code.into()]);
                            k = code_table[code.into()];
                        } else {
                            // eslint-disable-next-line prefer-destructuring
                            k = code_table[prev_code.into()];
                            index_stream.push(code_table[prev_code.into()]);
                            index_stream.push(k);
                        }
                        if (last_code < 0xFFF) {
                            last_code += 1;
                            code_table[last_code.into()] = [code_table[prev_code], k];
                            if (last_code == grow_code && last_code < 0xFFF) {
                                size += 1;
                                grow_code = (2 << size - 1) - 1;
                            }
                        }
                    }
                    if !br.hasBits(size) {
                        break;
                    }
                }
                self.increment_offset(1);
                data_sub_blocks_count -= 1;
            }
            data_sub_blocks_count = contents[self.offset];
            self.increment_offset(1);
            if data_sub_blocks_count == 0 {
                break;
            }
        }
    }
    fn handle_plain_text_extension(&mut self, gif: &mut Gif, contents: &[u8]) {
        // Plain Text Extension (Optional)
        let block_size: usize = contents[self.offset].into();
        self.increment_offset(1 + block_size);

        // Data sub block section
        let mut data_sub_blocks_count = contents[self.offset];
        self.increment_offset(1);
        loop {
            let mut data_sub_block;
            for n in 0..data_sub_blocks_count {
                data_sub_block = contents[self.offset];
                self.increment_offset(1);
            }
            data_sub_blocks_count = contents[self.offset];
            self.increment_offset(1);
            if data_sub_blocks_count == 0x00 {
                break;
            }
        }
    }
    fn handle_application_extension(&mut self, gif: &mut Gif, contents: &[u8]) {
        // Application Extension (Optional)
        let block_size: usize = contents[self.offset].into();
        self.increment_offset(1);

        let mut application = String::from("");
        let length = self.offset + block_size;
        match String::from_utf8(contents[self.offset..length].to_vec()) {
            Ok(parsed_application) => {
                application = parsed_application;
            }
            Err(err) => println!("Error 3: {}", err),
        }
        self.increment_offset(block_size);

        // Data sub block section
        let mut data_sub_blocks_count = contents[self.offset];
        self.increment_offset(1);
        loop {
            for n in 0..data_sub_blocks_count {
                let data_sub_block = contents[self.offset];
                self.increment_offset(1);
            }
            data_sub_blocks_count = contents[self.offset];
            self.increment_offset(1);
            if data_sub_blocks_count == 0 {
                break;
            }
        }
    }
    fn handle_comment_extension(&mut self, gif: &mut Gif, contents: &[u8]) {
        // Comment Extension (Optional)
        let mut data_sub_blocks_count = contents[self.offset];
        self.increment_offset(1);
        loop {
            for n in 0..data_sub_blocks_count {
                let data_sub_block = contents[self.offset];
                self.increment_offset(1);
            }
            data_sub_blocks_count = contents[self.offset];
            self.increment_offset(1);
            if data_sub_blocks_count == 0 {
                break;
            }
        }
    }
}

///

struct CodeUnit {
    stream: Vec<u8>,
    table: Vec<Option<u8>>,
    start: BitState
}
///

#[derive(Debug)]
pub enum GifError {
    SignatureError,
}

impl std::error::Error for GifError {}

impl fmt::Display for GifError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GifError::SignatureError => write!(f, "Signature Error"),
        }
    }
}