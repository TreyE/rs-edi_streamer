use std::io::Error;
use std::io::Read;
use crate::edi_constants::SEGMENT_STARTERS;

struct ParserConfig {
    element_delimiter: Vec<u8>,
//    sub_element_delimiter: Vec<u8>,
    segment_delimiter: Vec<u8>
}

#[allow(clippy::upper_case_acronyms)]
enum PState {
    InField,
    InSegTerm,
    EOF,
    Errored
}

struct ParserState {
    byte_index: u64,
    start_of_last_segment: u64,
    state: PState,
    segment_index: u64,
    current_string: Vec<u8>,
    current_field: Vec<u8>,
    current_segment: Vec<Vec<u8>>
}

pub struct ParserIterator<'a, T: Read> {
  io_source: &'a mut T,
  parser_state: ParserState,
  parser_config: ParserConfig
}

type ParserOutput = Option<Segment>;

pub struct Segment {
  pub tag: Vec<u8>,
  pub fields: Vec<Vec<u8>>,
  pub start_offset: u64,
  pub end_offset: u64,
  pub segment_index: u64,
  pub raw: Vec<u8>
}

pub fn create_segment_iterator<T: Read>(ioish: &mut T, element_delimiter: Vec<u8>, segment_delimiter: Vec<u8>) -> ParserIterator<T> {
  let pc = ParserConfig {
    element_delimiter,
    segment_delimiter
  };
  let ps = ParserState {
    byte_index: 0,
    start_of_last_segment: 0,
    segment_index: 0,
    state: PState::InField,
    current_string: Vec::new(),
    current_field: Vec::new(),
    current_segment: Vec::new()
  };
  ParserIterator {
    io_source: ioish,
    parser_config: pc,
    parser_state: ps
  }
}

impl<'a, T: Read> Iterator for ParserIterator<'a, T> {
  type Item = Result<Segment, Error>;

  fn next(&mut self) -> Option<Self::Item> {
    parser_next(self)
  }
}

fn parser_next<T: Read>(pi: &mut ParserIterator<T>) -> Option<Result<Segment, Error>> {
  match pi.parser_state.state {
    PState::Errored => None,
    PState::EOF => None,
    _ => {
      let res = step(&pi.parser_config, &mut pi.parser_state, &mut pi.io_source);
      match res {
        Ok(None) => parser_next(pi),
        Ok(Some(res)) => Some(Ok(res)),
        Err(e) => {
          pi.parser_state.state = PState::Errored;
          Some(Err(e))
        }
      }
    }
  }
}

fn build_segment(fields: Vec<Vec<u8>>, raw: Vec<u8>, start_index: u64, end_index: u64, segment_index: u64) -> Segment {
  let tag : Vec<u8> = match fields.get(0) {
    None => Vec::new(),
    Some(x) => x.clone()
  };
  Segment {
    tag,
    fields,
    start_offset: start_index,
    end_offset: end_index,
    segment_index,
    raw
  }
}

fn step<T: Read>(pc: &ParserConfig, ps: &mut ParserState, ioish: &mut T) -> Result<ParserOutput, Error> {
  let mut buff = [0; 1];
  let current_index = ps.byte_index;
  ps.byte_index += 1;
  match ioish.read(&mut buff) {
    Ok(size) if size == 1 => (),
    Ok(_) => {
      ps.state = PState::EOF;
      match ps.state {
        PState::InSegTerm => (),
        _ => {
          ps.current_segment.push(ps.current_field.clone());
        }
      }
      let s = ps.current_segment.clone();
      let seg = build_segment(s, ps.current_string.clone(), ps.start_of_last_segment, current_index, ps.segment_index);
      return Ok(Some(seg))
    },
    Err(e) => {
      return Err(e)
    }
  };

  let ed = pc.element_delimiter[0];
  // let sed = pc.sub_element_delimiter[0];
  let sd = pc.segment_delimiter[0];
  match buff[0] {
    x if x == ed => {
        ps.state = PState::InField;
        ps.current_string.push(x);
        let f = ps.current_field.clone();
        ps.current_field.clear();
        ps.current_segment.push(f);
        Ok(None)
    },
    z if z == sd => {
      ps.state = PState::InSegTerm;
      ps.current_string.push(z);
      let f = ps.current_field.clone();
      ps.current_segment.push(f);
      Ok(None)
    },
    a => {
        match ps.state {
          PState::InSegTerm if SEGMENT_STARTERS.contains(&a) => {
                let ns = ps.current_segment.clone();
                let seg = build_segment(ns, ps.current_string.clone(), ps.start_of_last_segment, current_index - 1, ps.segment_index);
                ps.current_string.clear();
                ps.current_field.clear();
                ps.current_segment.clear();
                ps.current_string.push(a);
                ps.current_field.push(a);
                ps.state = PState::InField;
                ps.start_of_last_segment = current_index;
                ps.segment_index +=  1;
                Ok(Some(seg))
          },
          PState::InSegTerm => {
            ps.current_string.push(a);
            Ok(None)
          },
          _ => {
            ps.current_string.push(a);
            ps.current_field.push(a);
            Ok(
              None
            )
          }
        }
    }
  }
}

#[cfg(test)]
mod test {
    use super::ParserConfig;
    use super::ParserState;
    use super::ParserIterator;
    use super::PState;
    use super::step;
    use std::io::Cursor;

    fn vectorize_string_for_compare(vec_string : &str) -> Vec<u8> {
      Vec::from(vec_string.as_bytes())
    }

    #[test]
    fn multi_step_test() {
      let mut ioish = Cursor::new("ISA".as_bytes());
      let config = ParserConfig {
        segment_delimiter: "~\n".bytes().collect(),
        // sub_element_delimiter: "^".bytes().collect(),
        element_delimiter: "*".bytes().collect()
      };
      let mut start = ParserState {
        byte_index: 0,
        start_of_last_segment: 0,
        segment_index: 0,
        state: PState::InField,
        current_string: Vec::new(),
        current_field: Vec::new(),
        current_segment: Vec::new()
      };
      let expected_vec =  
        Vec::from([vectorize_string_for_compare("ISA")]);
      _ = step(&config, &mut start, &mut ioish);
      _ = step(&config, &mut start, &mut ioish);
      _ = step(&config, &mut start, &mut ioish);
      match step(&config, &mut start, &mut ioish) {
        Ok(x) => {
          match x {
            Some(seg) => assert_eq!(seg.fields, expected_vec),
            None => {
              panic!("Error")
            }
          }
        }
        Err(_x) => {
            panic!("Error")
        }
      }
    }

    #[test]
    fn first_iteration_test() {
      let mut ioish = Cursor::new("ISA*ABCD~GS".as_bytes());
      let config = ParserConfig {
        segment_delimiter: "~\n".bytes().collect(),
        // sub_element_delimiter: "^".bytes().collect(),
        element_delimiter: "*".bytes().collect()
      };
      let start = ParserState {
        byte_index: 0,
        start_of_last_segment: 0,
        segment_index: 0,
        state: PState::InField,
        current_string: Vec::new(),
        current_field: Vec::new(),
        current_segment: Vec::new()
      };
      let mut pi = ParserIterator {
        parser_config: config,
        parser_state: start,
        io_source: &mut ioish
      };
      let result = pi.next();
      match result {
        None => panic!("NOTHING"),
        Some(x) => {
          let expected_vec =  
              Vec::from([
                  vectorize_string_for_compare("ISA"),
                  vectorize_string_for_compare("ABCD")
                ]);
          match x {
            Ok(y) => {
              assert_eq!(y.fields, expected_vec);
              assert_eq!(y.start_offset, 0);
              assert_eq!(y.end_offset, 8);
            }
            Err(_) => panic!("Wrong thing")
          }
        }
      }
    }
}
