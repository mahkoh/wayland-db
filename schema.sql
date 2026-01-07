create table repo
(
    repo_id bigint primary key,
    name    text not null,
    url     text not null
);

create table description
(
    description_id bigint primary key,
    summary        text,
    body           text not null
);

create table protocol
(
    protocol_id    bigint primary key,
    repo_id        bigint not null references repo,
    name           text   not null,
    path           text   not null,
    copyright      text,
    description_id bigint references description
);

create table interface
(
    interface_id   bigint primary key,
    protocol_id    bigint not null references protocol,
    name           text   not null,
    version        bigint not null,
    description_id bigint references description
);

create table enum
(
    enum_id        bigint primary key,
    interface_id   bigint  not null references interface,
    name           text    not null,
    since          bigint,
    is_bitfield    boolean not null,
    description_id bigint references description
);

create table entry
(
    entry_id         bigint primary key,
    enum_id          bigint not null references enum,
    name             text   not null,
    value_str        text   not null,
    value            bigint not null,
    summary          text,
    since            bigint,
    deprecated_since bigint,
    description_id   bigint references description
);

create table message
(
    message_id       bigint primary key,
    interface_id     bigint  not null references interface,
    number           bigint  not null,
    name             text    not null,
    is_request       boolean not null,
    is_destructor    boolean not null,
    since            bigint,
    deprecated_since bigint,
    description_id   bigint references description
);

create table type
(
    type_id bigint primary key,
    name    text not null
);

create table arg
(
    arg_id         bigint primary key,
    message_id     bigint  not null references message,
    position       bigint  not null,
    name           text    not null,
    type_id        bigint  not null references type,
    summary        text,
    description_id bigint references description,
    interface_name text,
    allow_null     boolean not null,
    enum_name      text
);

create index arg_type_id on arg (type_id);

create table rel_arg_interface
(
    arg_id       bigint not null references arg,
    interface_id bigint not null references interface
);

create index rel_arg_interface_arg_id on rel_arg_interface (arg_id);

create index rel_arg_interface_interface_id on rel_arg_interface (interface_id);

create table rel_arg_enum
(
    arg_id  bigint not null references arg,
    enum_id bigint not null references enum
);

create index rel_arg_enum_arg_id on rel_arg_enum (arg_id);

create index rel_arg_enum_enum_id on rel_arg_enum (enum_id);
