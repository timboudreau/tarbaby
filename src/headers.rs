/*
Copyright (C) 2026 Tim Boudreau

This program is free software; you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation; either version 2 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program; if not, write to the Free Software
Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.
*/
use rand::{Rng as _, rngs::ThreadRng};

/// To avoid allocation at runtime, build.rs simply generates 2048 random ascii alphanumeric
/// characters into a file in the build directory; we embed the bytes of it in the binary,
/// and when we need a random string we pick a random starting point in that with enough room
/// to supply the needed characters.
const ALPHANUM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/random_alphanumeric.txt"));

/// Generates a single nonsense header line, with some variation in element lengths,
/// such as `OarK-TwTWGdAU: qCsJYzbQxCOcwwWdeg`.  Things that are legal HTTP headers but
/// are simply noise.
pub fn random_header_line(result: &mut Vec<u8>) {
    let mut rng = rand::rng();
    random_string(random_length(3, 8, &mut rng), result, &mut rng);
    result.push(b'-');
    random_string(random_length(4, 9, &mut rng), result, &mut rng);
    result.extend_from_slice(b": ");
    random_string(random_length(9, 24, &mut rng), result, &mut rng);
    result.extend_from_slice(b"\r\n");
}

fn random_bytes(len: usize, rng: &mut ThreadRng) -> &'static [u8] {
    let pos = (rng.next_u32() as usize) % (ALPHANUM.len() - len);
    &ALPHANUM[pos..(pos + len)]
}

fn random_length(min: usize, max: usize, rng: &mut ThreadRng) -> usize {
    min + (rng.next_u32() as usize) % (max - min)
}

fn random_string(n: usize, result: &mut Vec<u8>, rng: &mut ThreadRng) {
    result.extend_from_slice(random_bytes(n, rng));
}
