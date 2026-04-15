# RlToml: Kaitai Struct definition for RealLive GAN format
# Copyright (C) 2026 Lucas Velasco

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
  id: gan_parser
  title: "Reallive GAN Format"
  application: "rltoml"
  file-extension: gan
  license: "GPL-3.0-or-later"
  ks-version: 0.11
  encoding: UTF-8
  endian: le

seq:
  - id: header
    type: header
  - id: data_section
    type: data_section

types:
  header:
    seq:
      - id: magic_1
        type: u4
        valid: 10000

      - id: magic_2
        type: u4
        valid: 10000

      - id: magic_3
        type: u4
        valid: 10100

      - id: bitmap_name_len
        type: u4

      - id: bitmap_name
        type: strz

  data_section:
    seq:
      - id: data_section_marker
        type: u4
        valid: 20000

      - id: num_sets
        type: u4

      - id: sets
        type: animation_set
        repeat: expr
        repeat-expr: num_sets
    
    types:
      animation_set:
        seq:
          - id: set_marker
            type: u4
            valid: 30000

          - id: num_frames
            type: u4

          - id: frames
            type: animation_frame
            repeat: expr
            repeat-expr: num_frames

      animation_frame:
        seq:
          - id: entries
            type: frame_entry
            repeat: until
            repeat-until: _.tag == 999999

      frame_entry:
        seq:
          - id: tag
            type: u4

          - id: value
            type: s4
            if: tag != 999999
