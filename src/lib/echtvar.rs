use crate::var32;
use crate::fields;
use std::io;
use std::io::prelude::*;
use std::fs;

use byteorder::{LittleEndian, ReadBytesExt};

use stream_vbyte::{
    decode::decode,
    x86::Ssse3
};

#[derive(Clone, Default, Debug, PartialEq, PartialOrd)]
pub struct EchtVar<T> {
    pub name: String,
    pub missing: i32,
    pub values: Vec<T>,
}

#[derive(Debug)]
pub struct EchtVars {
    pub zip: zip::ZipArchive<std::fs::File>,
    pub chrom: String,
    pub start: u32,
    pub var32s: Vec<var32::Var32>,
    pub longs: Vec<var32::LongVariant>,
    pub ints: Vec<EchtVar<u32>>,
    pub floats: Vec<EchtVar<f32>>,
    buffer: Vec<u8>,
}

impl EchtVars {

    pub fn open(path: &str) -> Self {
        let ep = std::path::Path::new(&*path);
        let file = fs::File::open(ep).expect("error accessing zip file");
        let mut result = EchtVars {
            zip: zip::ZipArchive::new(file).expect("error opening zip file"),
            chrom: "".to_string(),
            start: u32::MAX,
            var32s: vec![],
            longs: vec![],
            ints: vec![],
            floats: vec![],
            buffer: vec![],
        };

        {
            let mut f = result.zip.by_name("echtvar/config.json").expect("unable to open echtvar/config.json");
            let mut contents = String::new();
            f.read_to_string(&mut contents).expect("eror reading config.json");
            let flds: Vec<fields::Field> = json5::from_str(&contents).unwrap();
            eprintln!("fields: {:?}", flds);
            for fld in flds {
                result.ints.push(EchtVar::<u32>{
                    missing: fld.missing_value as i32, 
                    name: fld.alias,
                    values: vec![],
                });
    
            }
        }
        result
    }

    #[inline(always)]
    pub fn set_position(self: &mut EchtVars, chromosome: String, position: u32) -> io::Result<()> {
        if chromosome == self.chrom && position >> 20 == self.start >> 20 {
            return Ok(())
        }
        self.start = position >> 20 << 20; // round to 20 bits.
        self.chrom = chromosome;
        let base_path = format!("echtvar/{}/{}", self.chrom, position >> 20);
        eprintln!("base-path:{}", base_path);

        for fi in &mut self.ints {
            let path = format!("{}/{}.bin", base_path, fi.name);
            //eprintln!("path:{}", path);
            let mut iz = self.zip.by_name(&path)?;
            let n = iz.read_u32::<LittleEndian>()? as usize;
            //eprintln!("n:{}", n);
            self.buffer.resize(iz.size() as usize - std::mem::size_of::<u32>(), 0x0);
            iz.read_exact(&mut self.buffer)?;
            fi.values.resize(n, 0x0);
            // TODO: use skip to first position.
            let bytes_decoded = decode::<Ssse3>(&self.buffer, n, &mut fi.values);
            
            if bytes_decoded != self.buffer.len() {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "didn't read expected number of values from zip"))
            }
        }

        let long_path = format!("{}/too-long-for-var32.txt", base_path);
        let mut iz = self.zip.by_name(&long_path)?;
        self.buffer.clear();
        iz.read_to_end(&mut self.buffer)?;
        self.longs = serde_json::from_slice(&self.buffer)?;

        Ok(())


    }


}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_read() {
        let mut e = EchtVars::open("ec.zip");
        e.set_position("chr21".to_string(), 5030088).ok();

        assert_eq!(e.ints.len(), 2);
        assert_eq!(e.ints[0].values.len(), 46881);

        assert_eq!(e.longs[0].position, 5030185);

    }
}
