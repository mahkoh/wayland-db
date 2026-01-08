use {
    crate::{
        ast::{ArgType, Description, Interface, MessageType},
        collector::collect,
    },
    indexmap::IndexMap,
    linearize::{StaticMap, static_map},
    rusqlite::{Transaction, config::DbConfig, params},
    std::collections::HashMap,
    thiserror::Error,
};

#[derive(Debug, Error)]
pub enum GeneratorError {
    #[error("could not open {}", WAYLAND_DB)]
    OpenWaylandDb(#[source] rusqlite::Error),
    #[error("could not create a transaction")]
    CreateTransaction(#[source] rusqlite::Error),
    #[error("could not commit a transaction")]
    CommitTransaction(#[source] rusqlite::Error),
    #[error("could not reset the database")]
    ResetDb(#[source] rusqlite::Error),
    #[error("could not create the schema")]
    CreateSchema(#[source] rusqlite::Error),
    #[error("could not prepare statement {0}")]
    PrepareStatement(&'static str, #[source] rusqlite::Error),
    #[error("could not create a type")]
    InsertType(#[source] rusqlite::Error),
    #[error("could not insert a repo")]
    InsertRepo(#[source] rusqlite::Error),
    #[error("could not insert a description")]
    InsertDescription(#[source] rusqlite::Error),
    #[error("could not insert a protocol")]
    InsertProtocol(#[source] rusqlite::Error),
    #[error("could not insert an interface")]
    InsertInterface(#[source] rusqlite::Error),
    #[error("could not insert an enum")]
    InsertEnum(#[source] rusqlite::Error),
    #[error("could not insert an entry")]
    InsertEntry(#[source] rusqlite::Error),
    #[error("could not insert a message")]
    InsertMessage(#[source] rusqlite::Error),
    #[error("could not insert an arg")]
    InsertArg(#[source] rusqlite::Error),
    #[error("could not insert a rel_arg_interface")]
    InsertRelArgInterface(#[source] rusqlite::Error),
    #[error("could not insert a rel_arg_enum")]
    InsertRelArgEnum(#[source] rusqlite::Error),
    #[error("could not optimize the database")]
    OptimizeDatabase(#[source] rusqlite::Error),
}

const WAYLAND_DB: &str = "wayland.db";

pub fn main() -> Result<(), GeneratorError> {
    let mut db = rusqlite::Connection::open(WAYLAND_DB).map_err(GeneratorError::OpenWaylandDb)?;
    (|| {
        db.set_db_config(DbConfig::SQLITE_DBCONFIG_RESET_DATABASE, true)?;
        db.execute_batch("vacuum")?;
        db.set_db_config(DbConfig::SQLITE_DBCONFIG_RESET_DATABASE, false)
    })()
    .map_err(GeneratorError::ResetDb)?;
    let tx = db
        .transaction()
        .map_err(GeneratorError::CreateTransaction)?;
    insert(&tx)?;
    tx.commit().map_err(GeneratorError::CommitTransaction)?;
    db.execute_batch("pragma optimize")
        .map_err(GeneratorError::OptimizeDatabase)?;
    Ok(())
}

fn insert(tx: &Transaction<'_>) -> Result<(), GeneratorError> {
    let repos = collect();

    let mut next_id = 1i64;
    let mut next_id = || {
        let id = next_id;
        next_id += 1;
        id
    };

    tx.execute_batch(include_str!("../schema.sql"))
        .map_err(GeneratorError::CreateSchema)?;

    let prepare = |s: &'static str| {
        tx.prepare(s)
            .map_err(|e| GeneratorError::PrepareStatement(s, e))
    };

    // language=sqlite
    let mut insert_type = prepare("insert into type (type_id, name) values (?, ?)")?;
    // language=sqlite
    let mut insert_repo = prepare("insert into repo (repo_id, name, url) values (?, ?, ?)")?;
    // language=sqlite
    let mut insert_description =
        prepare("insert into description (description_id, summary, body) values (?, ?, ?)")?;
    // language=sqlite
    let mut insert_protocol = prepare(
        "insert into protocol \
         (protocol_id, repo_id, name, path, copyright, description_id) \
         values \
         (?, ?, ?, ?, ?, ?)",
    )?;
    // language=sqlite
    let mut insert_interface = prepare(
        "insert into interface \
         (interface_id, protocol_id, name, version, description_id) \
         values \
         (?, ?, ?, ?, ?)",
    )?;
    // language=sqlite
    let mut insert_enum = prepare(
        "insert into enum \
         (enum_id, interface_id, name, since, is_bitfield, description_id) \
         values \
         (?, ?, ?, ?, ?, ?)",
    )?;
    // language=sqlite
    let mut insert_entry = prepare(
        "insert into entry \
         (entry_id, enum_id, name, value_str, value, summary, since, deprecated_since, description_id) \
         values \
         (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )?;
    // language=sqlite
    let mut insert_message = prepare(
        "insert into message \
         (message_id, interface_id, number, name, is_request, is_destructor, since, deprecated_since, description_id) \
         values \
         (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )?;
    // language=sqlite
    let mut insert_arg = prepare(
        "insert into arg \
         (arg_id, message_id, position, name, type_id, summary, description_id, interface_name, allow_null, enum_name) \
         values \
         (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )?;
    // language=sqlite
    let mut insert_rel_arg_interface = prepare(
        "insert into rel_arg_interface \
         (arg_id, interface_id) \
         values \
         (?, ?)",
    )?;
    // language=sqlite
    let mut insert_rel_arg_enum = prepare(
        "insert into rel_arg_enum \
         (arg_id, enum_id) \
         values \
         (?, ?)",
    )?;

    let mut insert_description = |id: i64, description: &Description| {
        insert_description
            .execute(params![
                id,
                &description.summary,
                format_description(&description.body)
            ])
            .map_err(GeneratorError::InsertDescription)
    };
    macro_rules! insert_description {
        ($description:expr) => {{
            let mut description_id = None;
            if let Some(description) = $description {
                let id = next_id();
                description_id = Some(id);
                insert_description(id, description)?;
            }
            description_id
        }};
    }

    let types: StaticMap<ArgType, _> = static_map! {
        ty => {
            let id = next_id();
            let name = match ty {
                ArgType::NewId => "new_id",
                ArgType::Int => "int",
                ArgType::Uint => "uint",
                ArgType::Fixed => "fixed",
                ArgType::String => "string",
                ArgType::Object => "object",
                ArgType::Array => "array",
                ArgType::Fd => "fd",
            };
            insert_type
                .execute(params![id, name])
                .map_err(GeneratorError::InsertType)?;
            id
        }
    };

    let mut interface_deps: IndexMap<&str, InterfaceDep<'_>> = Default::default();
    #[derive(Default, Debug)]
    struct InterfaceDep<'a> {
        interface_ids: Vec<i64>,
        enums: IndexMap<&'a str, EnumDep>,
        arg_ids: Vec<i64>,
    }
    #[derive(Default, Debug)]
    struct EnumDep {
        enum_ids: Vec<i64>,
        arg_ids: Vec<i64>,
    }

    for repo in &repos {
        let repo_id = next_id();
        insert_repo
            .execute(params![repo_id, &repo.name, repo.url.trim()])
            .map_err(GeneratorError::InsertRepo)?;
        for protocol in &repo.protocols {
            struct LocalInterface<'a> {
                interface_id: i64,
                interface: &'a Interface,
                enums: HashMap<&'a str, i64>,
            }
            let mut interfaces: IndexMap<&str, LocalInterface<'_>> = Default::default();
            let protocol_id = next_id();
            let description_id = insert_description!(&protocol.description);
            insert_protocol
                .execute(params![
                    protocol_id,
                    repo_id,
                    &protocol.name,
                    &protocol.path,
                    protocol.copyright.as_ref().map(|c| &c.body),
                    description_id,
                ])
                .map_err(GeneratorError::InsertProtocol)?;
            for interface in &protocol.interfaces {
                let interface_id = next_id();
                let interface_dep = interface_deps.entry(&interface.name).or_default();
                interface_dep.interface_ids.push(interface_id);
                let local_interface = interfaces
                    .entry(&interface.name)
                    .insert_entry(LocalInterface {
                        interface_id,
                        interface,
                        enums: Default::default(),
                    })
                    .into_mut();
                let description_id = insert_description!(&interface.description);
                insert_interface
                    .execute(params![
                        interface_id,
                        protocol_id,
                        &interface.name,
                        interface.version as i64,
                        description_id,
                    ])
                    .map_err(GeneratorError::InsertInterface)?;
                for enum_ in &interface.enums {
                    let enum_id = next_id();
                    interface_dep
                        .enums
                        .entry(&enum_.name)
                        .or_default()
                        .enum_ids
                        .push(enum_id);
                    local_interface.enums.insert(&enum_.name, enum_id);
                    let description_id = insert_description!(&enum_.description);
                    insert_enum
                        .execute(params![
                            enum_id,
                            interface_id,
                            &enum_.name,
                            enum_.since.map(|v| v as i64),
                            enum_.bitfield,
                            description_id,
                        ])
                        .map_err(GeneratorError::InsertEnum)?;
                    for entry in &enum_.entries {
                        let entry_id = next_id();
                        let description_id = insert_description!(&entry.description);
                        insert_entry
                            .execute(params![
                                entry_id,
                                enum_id,
                                &entry.name,
                                &entry.value,
                                entry.value_i64,
                                &entry.summary,
                                entry.since,
                                entry.deprecated_since,
                                description_id,
                            ])
                            .map_err(GeneratorError::InsertEntry)?;
                    }
                }
            }
            for interface in interfaces.values() {
                for message in &interface.interface.messages {
                    let message_id = next_id();
                    let description_id = insert_description!(&message.description);
                    insert_message
                        .execute(params![
                            message_id,
                            interface.interface_id,
                            message.message_id as i64,
                            &message.name,
                            message.is_request,
                            message.ty == Some(MessageType::Destructor),
                            message.since,
                            message.deprecated_since,
                            description_id,
                        ])
                        .map_err(GeneratorError::InsertMessage)?;
                    for (pos, arg) in message.args.iter().enumerate() {
                        let arg_id = next_id();
                        let description_id = insert_description!(&arg.description);
                        insert_arg
                            .execute(params![
                                arg_id,
                                message_id,
                                pos as i64,
                                &arg.name,
                                types[arg.ty],
                                &arg.summary,
                                description_id,
                                &arg.interface,
                                arg.allow_null,
                                &arg.enum_,
                            ])
                            .map_err(GeneratorError::InsertArg)?;
                        if let Some(interface_name) = &arg.interface {
                            if let Some(local_interface) = interfaces.get(&**interface_name) {
                                insert_rel_arg_interface
                                    .insert(params![arg_id, local_interface.interface_id])
                                    .map_err(GeneratorError::InsertRelArgInterface)?;
                            } else {
                                interface_deps
                                    .entry(interface_name)
                                    .or_default()
                                    .arg_ids
                                    .push(arg_id);
                            }
                        }
                        if let Some(enum_name) = &arg.enum_ {
                            let (interface_name, enum_name) = enum_name
                                .split_once(".")
                                .unwrap_or((&interface.interface.name, enum_name));
                            if let Some(local_interface) = interfaces.get(interface_name)
                                && let Some(enum_id) = local_interface.enums.get(enum_name)
                            {
                                insert_rel_arg_enum
                                    .insert(params![arg_id, enum_id])
                                    .map_err(GeneratorError::InsertRelArgEnum)?;
                            } else {
                                interface_deps
                                    .entry(interface_name)
                                    .or_default()
                                    .enums
                                    .entry(enum_name)
                                    .or_default()
                                    .arg_ids
                                    .push(arg_id);
                            }
                        }
                    }
                }
            }
        }
    }

    for dep in interface_deps.values() {
        for &interface_id in &dep.interface_ids {
            for &arg_id in &dep.arg_ids {
                insert_rel_arg_interface
                    .insert(params![arg_id, interface_id])
                    .map_err(GeneratorError::InsertRelArgInterface)?;
            }
        }
        for enum_ in dep.enums.values() {
            for &enum_id in &enum_.enum_ids {
                for &arg_id in &enum_.arg_ids {
                    insert_rel_arg_enum
                        .insert(params![arg_id, enum_id])
                        .map_err(GeneratorError::InsertRelArgEnum)?;
                }
            }
        }
    }

    Ok(())
}

fn format_description(description: &str) -> String {
    let mut needs_newline = false;
    let mut trim = None;
    let mut empty_lines = 0;
    let mut out = String::new();
    'outer: for mut line in description.lines() {
        if trim.is_none() {
            let mut spaces = 0usize;
            'spaces: {
                for c in line.chars() {
                    if c == ' ' {
                        spaces += 1;
                    } else if c == '\t' {
                        spaces = (spaces + 8) & !7;
                    } else {
                        break 'spaces;
                    }
                }
                continue 'outer;
            }
            trim = Some(spaces);
        }
        let trim = trim.unwrap();
        let mut line_buf = String::new();
        if line.contains('\t') {
            let mut offset = 0;
            for c in line.chars() {
                if c == '\t' {
                    line_buf.push(' ');
                    offset += 1;
                    let delta = (-offset) & 7;
                    for _ in 0..delta {
                        line_buf.push(' ');
                    }
                    offset += delta;
                } else {
                    line_buf.push(c);
                    offset += 1;
                }
            }
            line = &line_buf;
        }
        let idx = 'idx: {
            let mut spaces = 0usize;
            for (idx, c) in line.char_indices() {
                if c == ' ' {
                    spaces += 1;
                } else {
                    break 'idx idx;
                }
                if spaces >= trim {
                    break 'idx idx + 1;
                }
            }
            line.len()
        };
        line = &line[idx..];
        if line.trim_ascii().is_empty() {
            empty_lines += 1;
            continue;
        }
        if empty_lines > 0 {
            for _ in 0..empty_lines {
                out.push_str("\n");
            }
            empty_lines = 0;
        }
        if needs_newline {
            needs_newline = false;
            out.push_str("\n");
        }
        out.push_str(line);
    }
    out
}
