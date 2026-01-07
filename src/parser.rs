use {
    crate::ast::{
        Arg, ArgType, Copyright, Description, Entry, Enum, Interface, Message, MessageType,
        Protocol,
    },
    quick_xml::{
        Reader,
        events::{
            Event,
            attributes::{AttrError, Attribute, Attributes},
        },
    },
    std::{
        borrow::Cow,
        num::ParseIntError,
        path::Path,
        str::{FromStr, ParseBoolError},
        string::FromUtf8Error,
    },
    thiserror::Error,
};

#[derive(Debug, Error)]
pub enum ParserError {
    #[error("could not parse a protocol element")]
    Protocol(#[from] ProtocolError),
    #[error("could not read the next event")]
    ReadEvent(#[from] quick_xml::Error),
}

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("could not parse an attribute")]
    Attribute(#[from] AttributeError),
    #[error("protocol does not have a name")]
    MissingName,
    #[error("could not read the next event")]
    ReadEvent(#[from] quick_xml::Error),
    #[error("could not the copyright element")]
    Copyright(#[from] CopyrightError),
    #[error("could not parse the description element")]
    Description(#[from] DescriptionError),
    #[error("could not parse an interface element")]
    Interface(#[from] InterfaceError),
}

#[derive(Debug, Error)]
pub enum CopyrightError {
    #[error("could not read the next event")]
    ReadEvent(#[from] quick_xml::Error),
    #[error("could not decode the body as UTF-8")]
    DecodeUtf8(#[source] FromUtf8Error),
}

#[derive(Debug, Error)]
pub enum DescriptionError {
    #[error("could not parse an attribute")]
    Attribute(#[from] AttributeError),
    #[error("could not read the next event")]
    ReadEvent(#[from] quick_xml::Error),
    #[error("could not decode the body as UTF-8")]
    DecodeUtf8(#[source] FromUtf8Error),
}

#[derive(Debug, Error)]
pub enum InterfaceError {
    #[error("interface has no name")]
    MissingName,
    #[error("interface has no version")]
    MissingVersion,
    #[error("could not parse an attribute")]
    Attribute(#[from] AttributeError),
    #[error("could not read the next event")]
    ReadEvent(#[from] quick_xml::Error),
    #[error("could not parse the version")]
    Version(#[source] ParseIntError),
    #[error("could not parse a request element")]
    Request(#[source] MessageError),
    #[error("could not parse an event element")]
    Event(#[source] MessageError),
    #[error("could not parse the description element")]
    Description(#[from] DescriptionError),
    #[error("could not parse an enum element")]
    Enum(#[from] EnumError),
}

#[derive(Debug, Error)]
pub enum MessageError {
    #[error("message has no name")]
    MissingName,
    #[error("could not parse an attribute")]
    Attribute(#[from] AttributeError),
    #[error("could not read the next event")]
    ReadEvent(#[from] quick_xml::Error),
    #[error("could not parse the since attribute")]
    Since(#[source] ParseIntError),
    #[error("could not parse the deprecated-since attribute")]
    DeprecatedSince(#[source] ParseIntError),
    #[error("unknown message type {}", .0)]
    UnknownMessageType(String),
    #[error("could not parse an argument element")]
    Arg(#[from] ArgError),
    #[error("could not the description element")]
    Description(#[from] DescriptionError),
}

#[derive(Debug, Error)]
pub enum ArgError {
    #[error("argument has no name")]
    MissingName,
    #[error("argument has no type")]
    MissingType,
    #[error("could not parse an attribute")]
    Attribute(#[from] AttributeError),
    #[error("could not read the next event")]
    ReadEvent(#[from] quick_xml::Error),
    #[error("could not parse the allow-null attribute")]
    AllowNull(#[source] ParseBoolError),
    #[error("unknown arg type {}", .0)]
    UnknownArgType(String),
    #[error("could not the description element")]
    Description(#[from] DescriptionError),
}

#[derive(Debug, Error)]
pub enum EnumError {
    #[error("enum has no name")]
    MissingName,
    #[error("could not parse an attribute")]
    Attribute(#[from] AttributeError),
    #[error("could not read the next event")]
    ReadEvent(#[from] quick_xml::Error),
    #[error("could not parse the allow-null attribute")]
    AllowNull(#[source] ParseBoolError),
    #[error("could not the description element")]
    Description(#[from] DescriptionError),
    #[error("could not parse the since attribute")]
    Since(#[source] ParseIntError),
    #[error("could not an entry element")]
    Entry(#[from] EntryError),
}

#[derive(Debug, Error)]
pub enum EntryError {
    #[error("entry has no name")]
    MissingName,
    #[error("entry has no value")]
    MissingValue,
    #[error("value could not be parsed")]
    InvalidValue(#[source] ParseIntError),
    #[error("could not parse an attribute")]
    Attribute(#[from] AttributeError),
    #[error("could not read the next event")]
    ReadEvent(#[from] quick_xml::Error),
    #[error("could not the description element")]
    Description(#[from] DescriptionError),
    #[error("could not parse the since attribute")]
    Since(#[source] ParseIntError),
    #[error("could not parse the deprecated-since attribute")]
    DeprecatedSince(#[source] ParseIntError),
}

#[derive(Debug, Error)]
pub enum AttributeError {
    #[error("quick_xml returned an error")]
    QuickXml(#[from] AttrError),
    #[error("could not decode the value as UTF-8")]
    DecodeUtf8(#[from] quick_xml::Error),
}

pub(crate) fn parse(path: &Path, input: &[u8]) -> Result<Vec<Protocol>, ParserError> {
    let mut reader = Reader::from_reader(input);
    let mut protocols = Vec::new();
    loop {
        let event = reader.read_event().map_err(ParserError::ReadEvent)?;
        let (start, empty) = match event {
            Event::Start(s) => (s, false),
            Event::Empty(s) => (s, true),
            Event::Eof => break,
            _ => continue,
        };
        match start.local_name().as_ref() {
            b"protocol" => protocols.push(parse_protocol(
                path,
                &mut reader,
                start.attributes(),
                empty,
            )?),
            _ => continue,
        }
    }
    Ok(protocols)
}

macro_rules! parse_attr {
    ($attr:expr) => {
        match $attr {
            Ok(ref attr) => parse_attr(attr),
            Err(e) => return Err(AttributeError::QuickXml(e).into()),
        }
    };
}

fn parse_attr<'a>(attr: &'a Attribute) -> Result<(&'a [u8], Cow<'a, str>), AttributeError> {
    let name = attr.key.local_name().into_inner();
    let value = attr.unescape_value().map_err(AttributeError::DecodeUtf8)?;
    Ok((name, value))
}

fn parse_protocol(
    path: &Path,
    reader: &mut Reader<&[u8]>,
    attributes: Attributes,
    empty: bool,
) -> Result<Protocol, ProtocolError> {
    let mut name = None;
    for attr in attributes {
        let (n, value) = parse_attr!(attr)?;
        match n {
            b"name" => name = Some(value.into_owned()),
            _ => continue,
        }
    }
    let mut copyright = None;
    let mut description = None;
    let mut interfaces = vec![];
    if !empty {
        loop {
            let event = reader.read_event().map_err(ProtocolError::ReadEvent)?;
            let (start, empty) = match event {
                Event::Start(s) => (s, false),
                Event::End(_) => break,
                Event::Empty(s) => (s, true),
                _ => continue,
            };
            match start.local_name().as_ref() {
                b"copyright" => {
                    copyright = Some(parse_copyright(reader, start.attributes(), empty)?)
                }
                b"description" => {
                    description = Some(parse_description(reader, start.attributes(), empty)?)
                }
                b"interface" => {
                    interfaces.push(parse_interface(reader, start.attributes(), empty)?)
                }
                _ => continue,
            }
        }
    }
    let name = name.ok_or(ProtocolError::MissingName)?;
    Ok(Protocol {
        path: path.display().to_string(),
        name,
        copyright,
        description,
        interfaces,
    })
}

fn parse_copyright(
    reader: &mut Reader<&[u8]>,
    _attributes: Attributes,
    empty: bool,
) -> Result<Copyright, CopyrightError> {
    let mut body = Vec::new();
    if !empty {
        loop {
            let event = reader.read_event().map_err(CopyrightError::ReadEvent)?;
            match event {
                Event::Text(s) => body.extend_from_slice(s.as_ref()),
                Event::End(_) => break,
                _ => continue,
            }
        }
    }
    Ok(Copyright {
        body: String::from_utf8(body).map_err(CopyrightError::DecodeUtf8)?,
    })
}

fn parse_description(
    reader: &mut Reader<&[u8]>,
    attributes: Attributes,
    empty: bool,
) -> Result<Description, DescriptionError> {
    let mut summary = None;
    for attr in attributes {
        let (n, value) = parse_attr!(attr)?;
        match n {
            b"summary" => summary = Some(value.into_owned()),
            _ => continue,
        }
    }
    let mut body = Vec::new();
    if !empty {
        loop {
            let event = reader.read_event().map_err(DescriptionError::ReadEvent)?;
            match event {
                Event::Text(s) => body.extend_from_slice(s.as_ref()),
                Event::End(_) => break,
                _ => continue,
            }
        }
    }
    Ok(Description {
        summary,
        body: String::from_utf8(body).map_err(DescriptionError::DecodeUtf8)?,
    })
}

fn parse_interface(
    reader: &mut Reader<&[u8]>,
    attributes: Attributes,
    empty: bool,
) -> Result<Interface, InterfaceError> {
    let mut name = None;
    let mut version = None;
    for attr in attributes {
        let (n, value) = parse_attr!(attr)?;
        match n {
            b"name" => name = Some(value.into_owned()),
            b"version" => version = Some(value.parse().map_err(InterfaceError::Version)?),
            _ => continue,
        }
    }
    let mut description = None;
    let mut messages = Vec::new();
    let mut enums = Vec::new();
    if !empty {
        let mut num_requests = 0;
        let mut num_events = 0;
        loop {
            let event = reader.read_event().map_err(InterfaceError::ReadEvent)?;
            let (start, empty) = match event {
                Event::Start(s) => (s, false),
                Event::End(_) => break,
                Event::Empty(s) => (s, true),
                _ => continue,
            };
            match start.local_name().as_ref() {
                b"description" => {
                    description = Some(parse_description(reader, start.attributes(), empty)?)
                }
                b"request" => messages.push(
                    parse_message(reader, start.attributes(), empty, &mut num_requests, true)
                        .map_err(InterfaceError::Request)?,
                ),
                b"event" => messages.push(
                    parse_message(reader, start.attributes(), empty, &mut num_events, false)
                        .map_err(InterfaceError::Event)?,
                ),
                b"enum" => enums.push(parse_enum(reader, start.attributes(), empty)?),
                _ => continue,
            }
        }
    }
    let name = name.ok_or(InterfaceError::MissingName)?;
    Ok(Interface {
        name,
        version: version.ok_or(InterfaceError::MissingVersion)?,
        description,
        messages,
        enums,
    })
}

fn parse_message(
    reader: &mut Reader<&[u8]>,
    attributes: Attributes,
    empty: bool,
    message_ids: &mut usize,
    is_request: bool,
) -> Result<Message, MessageError> {
    let mut name = None;
    let mut ty = None;
    let mut since = None;
    let mut deprecated_since = None;
    for attr in attributes {
        let (n, value) = parse_attr!(attr)?;
        match n {
            b"name" => name = Some(value.into_owned()),
            b"type" => match value.as_ref() {
                "destructor" => ty = Some(MessageType::Destructor),
                _ => return Err(MessageError::UnknownMessageType(value.into_owned())),
            },
            b"since" => since = Some(value.parse().map_err(MessageError::Since)?),
            b"deprecated-since" => {
                deprecated_since = Some(value.parse().map_err(MessageError::DeprecatedSince)?)
            }
            _ => continue,
        }
    }
    let mut description = None;
    let mut args = Vec::new();
    if !empty {
        loop {
            let event = reader.read_event().map_err(MessageError::ReadEvent)?;
            let (start, empty) = match event {
                Event::Start(s) => (s, false),
                Event::End(_) => break,
                Event::Empty(s) => (s, true),
                _ => continue,
            };
            match start.local_name().as_ref() {
                b"description" => {
                    description = Some(parse_description(reader, start.attributes(), empty)?)
                }
                b"arg" => args.push(parse_arg(reader, start.attributes(), empty)?),
                _ => continue,
            }
        }
    }
    let message_id = *message_ids;
    *message_ids += 1;
    Ok(Message {
        name: name.ok_or(MessageError::MissingName)?,
        message_id,
        is_request,
        ty,
        since,
        deprecated_since,
        description,
        args,
    })
}

fn parse_arg(
    reader: &mut Reader<&[u8]>,
    attributes: Attributes,
    empty: bool,
) -> Result<Arg, ArgError> {
    let mut name = None;
    let mut ty = None;
    let mut summary = None;
    let mut interface = None;
    let mut allow_null = None;
    let mut enum_ = None;
    for attr in attributes {
        let (n, value) = parse_attr!(attr)?;
        match n {
            b"name" => name = Some(value.into_owned()),
            b"type" => {
                ty = Some(match value.as_ref() {
                    "int" => ArgType::Int,
                    "uint" => ArgType::Uint,
                    "fixed" => ArgType::Fixed,
                    "string" => ArgType::String,
                    "array" => ArgType::Array,
                    "fd" => ArgType::Fd,
                    "new_id" => ArgType::NewId,
                    "object" => ArgType::Object,
                    _ => return Err(ArgError::UnknownArgType(value.into_owned())),
                })
            }
            b"summary" => summary = Some(value.into_owned()),
            b"interface" => interface = Some(value.into_owned()),
            b"allow-null" => allow_null = Some(value.parse().map_err(ArgError::AllowNull)?),
            b"enum" => enum_ = Some(value.into_owned()),
            _ => continue,
        }
    }
    let mut description = None;
    if !empty {
        loop {
            let event = reader.read_event().map_err(ArgError::ReadEvent)?;
            let (start, empty) = match event {
                Event::Start(s) => (s, false),
                Event::End(_) => break,
                Event::Empty(s) => (s, true),
                _ => continue,
            };
            match start.local_name().as_ref() {
                b"description" => {
                    description = Some(parse_description(reader, start.attributes(), empty)?)
                }
                _ => continue,
            }
        }
    }
    Ok(Arg {
        name: name.ok_or(ArgError::MissingName)?,
        ty: ty.ok_or(ArgError::MissingType)?,
        summary,
        description,
        interface,
        allow_null: allow_null.unwrap_or_default(),
        enum_,
    })
}

fn parse_enum(
    reader: &mut Reader<&[u8]>,
    attributes: Attributes,
    empty: bool,
) -> Result<Enum, EnumError> {
    let mut name = None;
    let mut since = None;
    let mut bitfield = None;
    for attr in attributes {
        let (n, v) = parse_attr!(attr)?;
        match n {
            b"name" => name = Some(v.into_owned()),
            b"since" => since = Some(v.parse().map_err(EnumError::Since)?),
            b"bitfield" => bitfield = Some(v.parse().map_err(EnumError::AllowNull)?),
            _ => continue,
        }
    }
    let mut description = None;
    let mut entries = Vec::new();
    if !empty {
        loop {
            let event = reader.read_event().map_err(EnumError::ReadEvent)?;
            let (start, empty) = match event {
                Event::Start(s) => (s, false),
                Event::End(_) => break,
                Event::Empty(s) => (s, true),
                _ => continue,
            };
            match start.local_name().as_ref() {
                b"description" => {
                    description = Some(parse_description(reader, start.attributes(), empty)?)
                }
                b"entry" => entries.push(parse_entry(reader, start.attributes(), empty)?),
                _ => continue,
            }
        }
    }
    Ok(Enum {
        name: name.ok_or(EnumError::MissingName)?,
        since,
        bitfield: bitfield.unwrap_or_default(),
        description,
        entries,
    })
}

fn parse_entry(
    reader: &mut Reader<&[u8]>,
    attributes: Attributes,
    empty: bool,
) -> Result<Entry, EntryError> {
    let mut name = None;
    let mut value = None;
    let mut summary = None;
    let mut since = None;
    let mut deprecated_since = None;
    for attr in attributes {
        let (n, v) = parse_attr!(attr)?;
        match n {
            b"name" => name = Some(v.into_owned()),
            b"value" => value = Some(v.into_owned()),
            b"summary" => summary = Some(v.into_owned()),
            b"since" => since = Some(v.parse().map_err(EntryError::Since)?),
            b"deprecated-since" => {
                deprecated_since = Some(v.parse().map_err(EntryError::DeprecatedSince)?)
            }
            _ => continue,
        }
    }
    let mut description = None;
    if !empty {
        loop {
            let event = reader.read_event().map_err(EntryError::ReadEvent)?;
            let (start, empty) = match event {
                Event::Start(s) => (s, false),
                Event::End(_) => break,
                Event::Empty(s) => (s, true),
                _ => continue,
            };
            match start.local_name().as_ref() {
                b"description" => {
                    description = Some(parse_description(reader, start.attributes(), empty)?)
                }
                _ => continue,
            }
        }
    }
    let value_string = value.ok_or(EntryError::MissingValue)?;
    let mut negative = true;
    let mut value = &*value_string;
    if let Some(v) = value.strip_prefix("-") {
        negative = true;
        value = v;
    }
    let mut value_i64 = if let Some(value) = value.strip_prefix("0x") {
        i64::from_str_radix(value, 16).map_err(EntryError::InvalidValue)?
    } else {
        i64::from_str(value).map_err(EntryError::InvalidValue)?
    };
    if negative {
        value_i64 = -value_i64;
    }
    Ok(Entry {
        name: name.ok_or(EntryError::MissingName)?,
        value: value_string,
        value_i64,
        summary,
        since,
        deprecated_since,
        description,
    })
}
