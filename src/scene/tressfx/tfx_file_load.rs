use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};

use log::{info, trace};

use crate::scene::tressfx::tfx_file_data::TfxFileData;

pub fn load_tressfx_file<'a>(path: &std::path::Path) -> TfxFileData {
  info!("Loading TressFX asset from '{}'", path.to_string_lossy());

  let file = File::open(path).expect("Failed to open file");
  let mut r = BufReader::new(file);

  let version = read_float(&mut r);
  let num_hair_strands = read_uint(&mut r);
  let num_vertices_per_strand = read_uint(&mut r);
  let offset_vertex_position = read_uint(&mut r);
  let offset_strand_uv = read_uint(&mut r);
  let offset_vertex_uv = read_uint(&mut r);
  let offset_strand_thickness = read_uint(&mut r);
  let offset_vertex_color = read_uint(&mut r);

  let total_vertices = num_vertices_per_strand * num_hair_strands;
  let position_float_cnt = (total_vertices * 4) as usize;

  let mut tfx_data = TfxFileData {
    version,
    num_hair_strands,
    num_vertices_per_strand,
    offset_vertex_position,
    offset_strand_uv,
    offset_vertex_uv,
    offset_strand_thickness,
    offset_vertex_color,
    raw_vertex_positions: Vec::with_capacity(position_float_cnt),
  };
  trace!("{}", tfx_data);

  // load and coerce raw data into vectors
  read_float_array(
    &mut r,
    &mut tfx_data.raw_vertex_positions,
    tfx_data.offset_vertex_position as _,
    position_float_cnt,
  );

  tfx_data
}

fn read_uint<R: Read>(reader: &mut R) -> u32 {
  let mut buf = [0u8; std::mem::size_of::<u32>()];
  reader.read_exact(&mut buf).expect("Error read_uint");
  u32::from_le_bytes(buf)
}

fn read_float<R: Read>(reader: &mut R) -> f32 {
  let mut buf = [0u8; std::mem::size_of::<f32>()];
  reader.read_exact(&mut buf).expect("Error read_float");
  f32::from_le_bytes(buf)
}

fn read_float_array<R: Read + Seek>(
  reader: &mut R,
  target: &mut Vec<f32>,
  offset: u64,
  cnt: usize,
) {
  reader
    .seek(SeekFrom::Start(offset))
    .expect("Error read_float_array: seek");
  for _ in 0..cnt {
    target.push(read_float(reader));
  }
}
