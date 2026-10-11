use alloc::collections::BTreeSet;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

#[derive(Clone, Debug, PartialEq)]
pub enum Constant {
    Nil,
    Boolean(bool),
    Number(f32),
    String(Vec<u8>),
    Hash(u64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Instruction(pub u32);

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Function {
    pub upvalues: u32,
    pub parameters: u32,
    pub varargs: u8,
    pub registers: u32,
    pub instructions: Vec<Instruction>,
    pub constants: Vec<Constant>,
    pub metadata: [u32; 2],
    pub children: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Chunk {
    pub types: Vec<(u32, Vec<u8>)>,
    /// Root function is index zero; child indices address this same array.
    pub functions: Vec<Function>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    Header,
    Truncated,
    Count,
    String,
    Boolean,
    Constant(u8),
    Nesting,
    Trailing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error {
    pub at: usize,
    pub kind: ErrorKind,
}

pub fn parse(bytes: &[u8]) -> Result<Chunk, Error> {
    let mut reader = Reader {
        bytes,
        at: 0,
        functions: Vec::new(),
    };
    if reader.take(14)? != b"\x1bLua\x51\x0d\x01\x04\x04\x04\x04\x00\x00\x00" {
        return Err(reader.error(ErrorKind::Header));
    }
    let count = reader.count(8)?;
    let mut types = Vec::with_capacity(count);
    for _ in 0..count {
        let id = reader.word()?;
        types.push((id, reader.string()?));
    }
    reader.function(0)?;
    if reader.at != bytes.len() {
        return Err(reader.error(ErrorKind::Trailing));
    }
    Ok(Chunk {
        types,
        functions: reader.functions,
    })
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
    functions: Vec<Function>,
}

impl Reader<'_> {
    fn error(&self, kind: ErrorKind) -> Error {
        Error { at: self.at, kind }
    }

    fn take(&mut self, count: usize) -> Result<&[u8], Error> {
        let end = self
            .at
            .checked_add(count)
            .ok_or_else(|| self.error(ErrorKind::Count))?;
        let bytes = self
            .bytes
            .get(self.at..end)
            .ok_or_else(|| self.error(ErrorKind::Truncated))?;
        self.at = end;
        Ok(bytes)
    }

    fn byte(&mut self) -> Result<u8, Error> {
        Ok(self.take(1)?[0])
    }
    fn word(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn count(&mut self, minimum: usize) -> Result<usize, Error> {
        let count = self.word()? as usize;
        if count > (self.bytes.len() - self.at) / minimum {
            return Err(self.error(ErrorKind::Count));
        }
        Ok(count)
    }

    fn boolean(&mut self) -> Result<bool, Error> {
        match self.byte()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(self.error(ErrorKind::Boolean)),
        }
    }

    fn string(&mut self) -> Result<Vec<u8>, Error> {
        let length = self.word()? as usize;
        let bytes = self.take(length)?;
        let Some((&0, contents)) = bytes.split_last() else {
            return Err(self.error(ErrorKind::String));
        };
        Ok(contents.to_vec())
    }

    fn function(&mut self, depth: usize) -> Result<usize, Error> {
        if depth >= 64 {
            return Err(self.error(ErrorKind::Nesting));
        }
        let index = self.functions.len();
        self.functions.push(Function::default());
        let upvalues = self.word()?;
        let parameters = self.word()?;
        let varargs = self.byte()?;
        let registers = self.word()?;
        let count = self.count(4)?;
        let padding = (4 - self.at % 4) % 4;
        self.take(padding)?;
        let mut instructions = Vec::with_capacity(count);
        for _ in 0..count {
            instructions.push(Instruction(self.word()?));
        }
        let count = self.count(1)?;
        let mut constants = Vec::with_capacity(count);
        for _ in 0..count {
            let tag = self.byte()?;
            constants.push(match tag {
                0 => Constant::Nil,
                1 => Constant::Boolean(self.boolean()?),
                3 => Constant::Number(f32::from_bits(self.word()?)),
                4 => Constant::String(self.string()?),
                13 => Constant::Hash(u64::from_le_bytes(self.take(8)?.try_into().unwrap())),
                _ => return Err(self.error(ErrorKind::Constant(tag))),
            });
        }
        let metadata = [self.word()?, self.word()?];
        let count = self.count(33)?;
        let mut children = Vec::with_capacity(count);
        for _ in 0..count {
            children.push(self.function(depth + 1)?);
        }
        self.functions[index] = Function {
            upvalues,
            parameters,
            varargs,
            registers,
            instructions,
            constants,
            metadata,
            children,
        };
        Ok(index)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UiLiterals {
    pub modules: usize,
    pub strings: BTreeSet<String>,
    pub materials: BTreeSet<String>,
    pub errors: Vec<UiLiteralError>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiLiteralError {
    pub asset_index: usize,
    pub name: Option<String>,
    pub kind: UiLiteralErrorKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiLiteralErrorKind {
    RawFile(crate::RawFileError),
    Bytecode(Error),
}

impl crate::ZoneLoad {
    pub fn ui_literals(&self) -> UiLiterals {
        let mut out = UiLiterals::default();
        for (asset_index, asset) in self.assets.iter().enumerate() {
            if asset.ty == crate::AssetType::Material {
                if let Some(name) = asset
                    .header
                    .get(..4)
                    .and_then(|bytes| {
                        crate::walk::decode_ptr(u32::from_le_bytes(bytes.try_into().unwrap()))
                    })
                    .and_then(|at| self.blocks.cstr(at).ok())
                    .and_then(|bytes| core::str::from_utf8(bytes).ok())
                    .filter(|name| !name.starts_with(','))
                {
                    out.materials.insert(name.to_ascii_lowercase());
                }
                continue;
            }
            let raw = match crate::read_raw_file(self, asset) {
                Ok(Some(raw)) => raw,
                Ok(None) => continue,
                Err(error) => {
                    out.errors.push(UiLiteralError {
                        asset_index,
                        name: None,
                        kind: UiLiteralErrorKind::RawFile(error),
                    });
                    continue;
                }
            };
            if !(raw.name.starts_with("ui/") || raw.name.starts_with("ui_mp/"))
                || !raw.name.ends_with(".lua")
            {
                continue;
            }
            let chunk = match parse(raw.bytes) {
                Ok(chunk) => chunk,
                Err(error) => {
                    out.errors.push(UiLiteralError {
                        asset_index,
                        name: Some(raw.name.to_string()),
                        kind: UiLiteralErrorKind::Bytecode(error),
                    });
                    continue;
                }
            };
            out.modules += 1;
            for function in chunk.functions {
                for constant in function.constants {
                    if let Constant::String(bytes) = constant
                        && let Ok(name) = core::str::from_utf8(&bytes)
                    {
                        out.strings.insert(name.to_ascii_lowercase());
                    }
                }
            }
        }
        out
    }
}
