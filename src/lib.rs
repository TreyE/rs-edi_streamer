use std::io::Error;
use std::io::Read;

#[derive(Debug)]
struct ParserConfig {
    element_delimiter: Vec<u8>,
//    sub_element_delimiter: Vec<u8>,
    segment_delimiter: Vec<u8>
}

#[derive(Debug)]
enum PState {
    InField,
    InSegTerm,
    EOF,
    Errored
}

#[derive(Debug)]
struct ParserState {
    byte_index: u64,
    start_of_last_segment: u64,
    state: PState,
    segment_index: u64,
    current_string: Vec<u8>,
    current_field: Vec<u8>,
    current_segment: Vec<Vec<u8>>
}

#[derive(Debug)]
pub struct ParserIterator<T: Read> {
  io_source: T,
  parser_state: ParserState,
  parser_config: ParserConfig
}

#[derive(Debug)]
pub enum ParserError {
    IOError(Error)
}

static SEGMENT_STARTERS : [u8; 52] = [
    'a' as u8,
    'b' as u8,
    'c' as u8,
    'd' as u8,
    'e' as u8,
    'f' as u8,
    'g' as u8,
    'h' as u8,
    'i' as u8,
    'j' as u8,
    'k' as u8,
    'l' as u8,
    'm' as u8,
    'n' as u8,
    'o' as u8,
    'p' as u8,
    'q' as u8,
    'r' as u8,
    's' as u8,
    't' as u8,
    'u' as u8,
    'v' as u8,
    'w' as u8,
    'x' as u8,
    'y' as u8,
    'z' as u8,
    'A' as u8,
    'B' as u8,
    'C' as u8,
    'D' as u8,
    'E' as u8,
    'F' as u8,
    'G' as u8,
    'H' as u8,
    'I' as u8,
    'J' as u8,
    'K' as u8,
    'L' as u8,
    'M' as u8,
    'N' as u8,
    'O' as u8,
    'P' as u8,
    'Q' as u8,
    'R' as u8,
    'S' as u8,
    'T' as u8,
    'U' as u8,
    'V' as u8,
    'W' as u8,
    'X' as u8,
    'Y' as u8,
    'Z' as u8
];

type ParserOutput = Option<Segment>;

#[derive(Debug)]
pub struct Segment {
  pub tag: Vec<u8>,
  pub fields: Vec<Vec<u8>>,
  pub start_offset: u64,
  pub end_offset: u64,
  pub segment_index: u64,
  pub raw: Vec<u8>
}

pub fn create_parser<T: Read>(ioish: T, element_delimiter: Vec<u8>, segment_delimiter: Vec<u8>) -> ParserIterator<T> {
  let pc = ParserConfig {
    element_delimiter: element_delimiter,
    segment_delimiter: segment_delimiter
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

impl<T: Read> Iterator for ParserIterator<T> {
  type Item = Result<Segment, ParserError>;

  fn next(&mut self) -> Option<Self::Item> {
    parser_next(self)
  }
}

fn parser_next<T: Read>(pi: &mut ParserIterator<T>) -> Option<Result<Segment, ParserError>> {
  match pi.parser_state.state {
    PState::Errored => None,
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
    tag: tag,
    fields: fields,
    start_offset: start_index,
    end_offset: end_index,
    segment_index: segment_index,
    raw: raw
  }
}

fn step<T: Read>(pc: &ParserConfig, ps: &mut ParserState, ioish: &mut T) -> Result<ParserOutput, ParserError> {
  let mut buff = [0; 1];
  let current_index = ps.byte_index.clone();
  println!("{:?}\n", current_index);
  ps.byte_index = ps.byte_index + 1;
  match ioish.read(&mut buff) {
    Ok(size) if size == 1 => (),
    Ok(_) => {
      ps.state = PState::EOF;
      ps.current_segment.push(ps.current_field.clone());
      let s = ps.current_segment.clone();
      let seg = build_segment(s, ps.current_string.clone(), ps.start_of_last_segment, current_index - 1, ps.segment_index);
      return Ok(Some(seg))
    },
    Err(e) => {
      return Err(ParserError::IOError(e))
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
                ps.segment_index = ps.segment_index + 1;
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
              println!("{:?}", x);
              panic!("Error")
            }
          }
        }
        Err(x) => {
            println!("{:?}", x);
            panic!("Error")
        }
      }
    }

    #[test]
    fn first_iteration_test() {
      let ioish = Cursor::new("ISA*ABCD~GS".as_bytes());
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
        io_source: ioish
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
