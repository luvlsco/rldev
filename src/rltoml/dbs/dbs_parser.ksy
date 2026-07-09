# RlToml: Kaitai Struct definition for RealLive DBS format
# Copyright (C) 2026 luvlsco

# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.

# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.

# You should have received a copy of the GNU General Public License
# along with this program.  If not, see <http://www.gnu.org/licenses/>.

meta:
  id: dbs_parser
  title: "RealLive DBS Format"
  application: "RlToml"
  file-extension: "dbs"
  license: "GPL-3.0-or-later"
  ks-version: 0.11
  encoding: Shift_JIS
  endian: le

doc-ref:
  - "https://github.com/rinrin-/crass/blob/master/cui/RealLive/RealLive.cpp#L678"

enums:
  column_type:
    0x53: string
    0x56: integer

seq:
  - id: string_end
    type: u4
  - id: num_row_ids
    type: u4
  - id: num_types
    type: u4
  - id: row_id_offset
    type: u4
  - id: type_list_offset
    type: u4
  - id: value_list_offset
    type: u4
  - id: string_table_offset
    type: u4

instances:
  row_ids:
    type: u4
    repeat: expr
    repeat-expr: num_row_ids
    pos: row_id_offset
    doc: "Sequential row IDs (0, 1, 2, ..., num_row_ids-1)"

  types:
    type: column_type_entry
    repeat: expr
    repeat-expr: num_types
    pos: type_list_offset
    doc: "Column type descriptors"

  values:
    type: u4
    repeat: expr
    repeat-expr: num_row_ids * num_types
    pos: value_list_offset
    doc: "Flat value array: string offsets (relative to string_table_offset) or raw integers"

  string_table_raw:
    size: string_end - string_table_offset
    pos: string_table_offset
    doc: "Raw string table bytes (SJIS, null-terminated)"

  rows:
    type: row(_index)
    repeat: expr
    repeat-expr: row_ids.size
    doc: "Rows with each cell resolved according to its column's data_type"

types:
  column_type_entry:
    seq:
      - id: column_id
        type: u4
        doc: "Column identifier"

      - id: data_type
        type: u4
        enum: column_type
        doc: "0x53 = string (SJIS), 0x56 = integer"

  row:
    params:
      - id: row_idx
        type: u4

    instances:
      cells:
        type: cell(row_idx, _index)
        repeat: expr
        repeat-expr: _root.num_types
        doc: "One cell per column, resolved (string or integer) via cell type"

  cell:
    params:
      - id: row_idx
        type: u4
      - id: col_idx
        type: u4

    instances:
      raw_value:
        value: "_root.values[row_idx * _root.num_types + col_idx]"
        doc: "Underlying flat u4 value before interpretation"

      col_type:
        value: _root.types[col_idx].data_type

      str_value:
        pos: _root.string_table_offset + raw_value
        type: strz
        io: _root._io
        if: col_type == column_type::string
        doc: "Resolved string when column data_type == string"

      int_value:
        value: raw_value
        if: col_type == column_type::integer
        doc: "Resolved integer when column data_type == integer"
