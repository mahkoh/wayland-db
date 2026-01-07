# wayland-db

This repository contains the `wayland.db` sqlite database that describes the
protocols from the following sources:

- https://gitlab.freedesktop.org/wayland/wayland.git
- https://gitlab.freedesktop.org/wayland/wayland-protocols.git
- https://gitlab.freedesktop.org/wlroots/wlr-protocols.git
- https://github.com/hyprwm/hyprland-protocols.git
- https://github.com/linuxdeepin/treeland-protocols.git
- https://github.com/mahkoh/jay-protocols.git
- https://invent.kde.org/libraries/plasma-wayland-protocols.git
- https://gitlab.freedesktop.org/wayland/weston.git
- https://github.com/pop-os/cosmic-protocols.git
- https://codeberg.org/river/river.git

The schema can be found in `schema.sql`. The database file is updated
automatically every 6 hours.

## Examples

Find all messages that reference `xdg_popup`:

```sqlite
select distinct
    r.name repo,
    p.name proto,
    i.name interface,
    m.name message,
    a.name arg,
    t.name type
from repo r
join protocol p using (repo_id)
join interface i using (protocol_id)
join message m using (interface_id)
join arg a using (message_id)
join type t using (type_id)
join rel_arg_interface rai using (arg_id)
join interface i2 on i2.interface_id = rai.interface_id
where i2.name = 'xdg_popup'
order by r.name, p.name, i.name, m.name, a.name;
```

```
+-----------------+---------------------------+------------------------+-------------+-----+------+
|repo             |proto                      |interface               |message      |arg  |type  |
+-----------------+---------------------------+------------------------+-------------+-----+------+
|jay-protocols    |jay_popup_ext_v1           |jay_popup_ext_manager_v1|get_ext      |popup|object|
|jay-protocols    |jay_tray_v1                |jay_tray_item_v1        |get_popup    |popup|object|
|wayland-protocols|xdg_shell                  |xdg_surface             |get_popup    |id   |new_id|
|wayland-protocols|xdg_shell_unstable_v5      |xdg_shell               |get_xdg_popup|id   |new_id|
|wlr-protocols    |wlr_layer_shell_unstable_v1|zwlr_layer_surface_v1   |get_popup    |popup|object|
+-----------------+---------------------------+------------------------+-------------+-----+------+
```

Find all inter-protocol dependencies in wayland-protocols:

```sqlite
select distinct
    p.name downstream,
    p2.name upstream
from repo r
join protocol p using (repo_id)
join interface i using (protocol_id)
join message m using (interface_id)
join arg a using (message_id)
join (
        select * from rel_arg_interface rai
    union
        select rae.arg_id, e.interface_id
        from rel_arg_enum rae
        join enum e using (enum_id)
) rai using (arg_id)
join interface i2 on i2.interface_id = rai.interface_id
join protocol p2 on p2.protocol_id = i2.protocol_id
where r.name = 'wayland-protocols'
  and p2.name != 'wayland'
  and p.protocol_id != p2.protocol_id
order by downstream, upstream;
```

```
+-------------------------------+----------------------------+
|downstream                     |upstream                    |
+-------------------------------+----------------------------+
|cursor_shape_v1                |tablet_unstable_v2          |
|cursor_shape_v1                |tablet_v2                   |
|ext_image_capture_source_v1    |ext_foreign_toplevel_list_v1|
|ext_image_copy_capture_v1      |ext_image_capture_source_v1 |
|input_method_experimental_v2   |xx_text_input_unstable_v3   |
|keyboard_filter_experimental_v1|input_method_experimental_v2|
|linux_dmabuf_unstable_v1       |linux_dmabuf_v1             |
|linux_dmabuf_v1                |linux_dmabuf_unstable_v1    |
|tablet_unstable_v2             |tablet_v2                   |
|tablet_v2                      |tablet_unstable_v2          |
|xdg_decoration_unstable_v1     |xdg_shell                   |
|xdg_dialog_v1                  |xdg_shell                   |
|xdg_shell                      |xdg_shell_unstable_v5       |
|xdg_shell_unstable_v5          |xdg_shell                   |
|xdg_toplevel_drag_v1           |xdg_shell                   |
|xdg_toplevel_icon_v1           |xdg_shell                   |
|xdg_toplevel_tag_v1            |xdg_shell                   |
|xx_session_management_v1       |xdg_shell                   |
+-------------------------------+----------------------------+
```

## Development

Development is done on the `master` branch. The default `db` branch that
contains the database is rebased automatically. To prevent the repository from
getting too large, only the latest version of the database is preserved.

## License

Everything in this repo other than the database itself is licensed under GPLv3.

The license of the database file itself is a combination of the licenses of the
source protocols. Use at your own risk.
