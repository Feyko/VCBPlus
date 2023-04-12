use byteorder::{BigEndian, ReadBytesExt};
use base64::{Engine as _, engine::general_purpose, engine};
use std::{
    io::{self, Result, Read, Cursor,ErrorKind::UnexpectedEof},
    fs
};
use std::collections::HashMap;
use std::io::Error;

#[macro_use] extern crate enum_primitive;
use enum_primitive::{enum_from_primitive, FromPrimitive};
use crate::Ink::{Invalid, Write};

fn main() {
    let b64_blueprint = fs::read_to_string("testdata/bp-all-logic")
        .expect("should be able to read file");
    println!("{}", b64_blueprint);

    let decoded: Vec<u8> = general_purpose::STANDARD.decode(b64_blueprint)
        .expect("should be base64");

    let data = BlueprintData::from_reader(Cursor::new(decoded))
        .expect("should be valid blueprint");
    println!("{:?}", data);
    let logic_data = &data.blocks[0].data;
    println!("{:X?}",logic_data);
    let inks = block_to_inks(&data, 0)
        .expect("should be valid inks");
    println!("{:?}", inks)
}

#[derive(Debug)]
struct BlueprintData {
    header: BlueprintHeader,
    blocks: Vec<BlockData>
}

impl BlueprintData {
    fn from_reader(mut rdr: impl Read) -> Result<Self> {
        let header = BlueprintHeader::from_reader(&mut rdr)?;
        let mut blocks = Vec::new();

        while let Some(block) = BlockData::from_reader(&mut rdr)? {
            blocks.push(block)
        }

        Ok(BlueprintData {
            header,
            blocks
        })
    }
}

#[derive(Debug)]
struct BlueprintHeader {
    version: u32,
    checksum: [u8; 6],
    width: u32,
    height: u32
}

impl BlueprintHeader {
    fn from_reader(rdr: &mut impl Read) -> Result<Self> {
        rdr.read_exact(&mut [0; 3]);

        let mut version_bytes: [u8; 3] = [0; 3];
        rdr.read_exact(&mut version_bytes);
        let version: u32 = u32_from_3_bytes_be(&version_bytes);

        let mut checksum: [u8; 6] = [0; 6];
        rdr.read_exact(&mut checksum)?;

        let width = rdr.read_u32::<BigEndian>()?;
        let height = rdr.read_u32::<BigEndian>()?;

        Ok(BlueprintHeader {
            version,
            checksum,
            width,
            height
        })
    }
}

fn u32_from_3_bytes_be(bytes: &[u8; 3]) -> u32 {
    return ((bytes[0] as u32) << 16) +
           ((bytes[1] as u32) <<  8) +
           ((bytes[2] as u32) <<  0)
}

#[derive(Debug)]
struct BlockData {
    size: u32,
    id: u32,
    data_size: u32,
    data: Vec<u8>
}

impl BlockData {
    fn from_reader(mut rdr: impl Read) -> Result<Option<Self>> {
        let size_check = rdr.read_u32::<BigEndian>();
        match size_check {
            Ok(_) => {}
            Err(e) if e.kind() == UnexpectedEof => {
                return Ok(None)
            }
            Err(e) => return Err(e)
        }
        let size = size_check?;
        let id = rdr.read_u32::<BigEndian>()?;
        let data_size = rdr.read_u32::<BigEndian>()?;

        let mut decoder = zstd::Decoder::new(rdr)?;
        let mut data = vec!(0; data_size as usize);
        decoder.read_exact(&mut data);

        Ok(Some(BlockData {
            size,
            id,
            data_size,
            data
        }))
    }
}

enum_from_primitive!{
    #[repr(u32)]
    #[derive(Debug)]
    #[derive(Clone)]
    enum Ink {
        Invalid = 0x69696969,

        Empty = 0x00000000,
        Write = 0xFF3E384D,
        Read = 0xFF5D472E,
        Cross = 0xFF8E7866,
        Trace = 0xFF5698A1,
        Buffer = 0xFF63FF92,
        And = 0xFF63C6FF,
        Or = 0xFFFFF263,
        Xor = 0xFFFF74AE,
        Not = 0xFF8A62FF,
        Nand = 0xFF00A2FF,
        Nor = 0xFFFFD930,
        Nxor = 0xFFFF00A6,
        LatchOn = 0xFF9FFF63,
        LatchOff = 0xFF474D38,
        Clock = 0xFF4100FF,
        LED = 0xFFFFFFFF
    }
}

fn block_to_inks(bp: &BlueprintData, block_index: usize) -> Result<Vec<Vec<Ink>>>{
    let block = &bp.blocks[block_index];
    let width = bp.header.width;
    let height = bp.header.height;

    let mut inks: Vec<Vec<Ink>> = vec!(vec!(Invalid; width as usize); height as usize);
    let mut chunks = block.data.chunks_exact(4);

    for y in 0..height as usize {
        for x in 0..width as usize {
            let chunk = chunks.next().unwrap();
            let chunk_u32 = unsafe {std::mem::transmute::<[u8; 4], u32>(chunk.try_into().unwrap())};
            let mut ink = Ink::from_u32(chunk_u32);
            if ink.is_none() {
                println!("Invalid ink value {:#X}", chunk_u32);
            }
            inks[y][x] = ink.unwrap();
        }
    }

    return Ok(inks)
}