use crate::edi_parsers::StreamParser;
use crate::edi_segments::Segment;
use std::sync::Arc;
use core::cell::RefCell;

#[derive(PartialEq)]
enum ParserState {
  Nothing,
  InInterchange,
  InFunctionalGroup,
  InTransaction
}

struct Interchange {
  functional_groups: Vec<Arc<FunctionalGroup>>,
  segments: Vec<Arc<Segment>>
}

struct FunctionalGroup {
  transactions: Vec<Arc<Transaction>>,
  segments: Vec<Arc<Segment>>
}

struct Transaction {
  segments: Vec<Arc<Segment>>
}

pub struct DefaultParser {
  state: ParserState,
  interchanges: Vec<Arc<Interchange>>,
  current_interchange: Option<RefCell<Interchange>>,
  current_functional_group: Option<RefCell<FunctionalGroup>>,
  current_transaction: Option<RefCell<Transaction>>,
  segments: Vec<Arc<Segment>>
}

impl DefaultParser {
  pub fn new() -> Self {
    DefaultParser {
      state: ParserState::Nothing,
      interchanges: Vec::new(),
      current_interchange: None,
      current_functional_group: None,
      current_transaction: None,
      segments: Vec::new()
    }
  }
}

impl<'a> StreamParser for DefaultParser {
  fn transaction_start(&mut self, _segment: &Segment) {
    let trans = Transaction {
      segments: Vec::new()
    };
    self.state = ParserState::InTransaction;
    let rc = RefCell::new(trans);
    self.current_transaction = Some(rc);
  }

  fn transaction_end(&mut self, _segment: Option<&Segment>) {
    self.state = ParserState::InFunctionalGroup;
    match &self.current_functional_group {
      None => (),
      Some(fg) => {
        match &self.current_transaction {
          None => (),
          Some(ct) => {
            let ctb = ct.borrow();
            let trans = Transaction {
              segments: ctb.segments.clone()
            };
            let trc = Arc::new(trans);
            let mut fgm = fg.borrow_mut();
            fgm.transactions.push(trc);
          }
        }
      }
    }
    self.current_transaction = None;
  }

  fn functional_group_start(&mut self, _segment: &Segment) {
    self.state = ParserState::InFunctionalGroup;
    let fg = FunctionalGroup {
      transactions: Vec::new(),
      segments: Vec::new()
    };
    let rc = RefCell::new(fg);
    self.current_functional_group = Some(
      rc
    )
  }

  fn functional_group_end(&mut self, _segment: Option<&Segment>) {
    self.state = ParserState::InInterchange;
    match &self.current_interchange {
      None => (),
      Some(ci) => {
        match &self.current_functional_group {
          None => (),
          Some(fg) => {
            let fgb = fg.borrow();
            let new_group = FunctionalGroup {
              transactions: fgb.transactions.clone(),
              segments: fgb.segments.clone()
            };
            let tfg = Arc::new(new_group);
            let mut cim = ci.borrow_mut();
            cim.functional_groups.push(tfg);
          }
        }
      }
    }
    self.current_functional_group = None;
  }

  fn interchange_start(&mut self, _segment: &Segment) {
    self.state = ParserState::InInterchange;
    let interchange = Interchange {
      functional_groups: Vec::new(),
      segments: Vec::new()
    };
    let rc = RefCell::new(interchange);
    self.current_interchange = Some(
      rc
    )
  }

  fn interchange_end(&mut self, _segment: Option<&Segment>) {
    self.state = ParserState::Nothing;
    match &self.current_interchange {
      None => (),
      Some(ci) => {
        let cim = ci.borrow();
        let interchange = Interchange {
          segments: cim.segments.clone(),
          functional_groups: cim.functional_groups.clone()
        };
        let interchange_rc = Arc::new(interchange);
        self.interchanges.push(interchange_rc);
      }
    }
    self.current_interchange = None;
  }

  fn error(&mut self, _error: std::io::Error) {

  }

  fn stream_end(&mut self) {

  }

  fn segment(&mut self, segment: &Segment) {
    consume_segment(self, segment);
  }

  fn in_interchange(&self) -> bool {
    (self.state == ParserState::InTransaction) || (self.state == ParserState::InFunctionalGroup) || (self.state == ParserState::InInterchange)
  }

  fn in_functional_group(&self) -> bool {
    (self.state == ParserState::InTransaction) || (self.state == ParserState::InFunctionalGroup)
  }

  fn in_transaction(&self) -> bool {
    self.state == ParserState::InTransaction      
  }
}

fn consume_segment(parser: &mut DefaultParser, segment: &Segment) {
  let new_seg: Segment = Segment {
    tag: segment.tag.clone(),
    fields: segment.fields.clone(),
    start_offset: segment.start_offset,
    end_offset: segment.end_offset,
    segment_index: segment.segment_index,
    raw: segment.raw.clone()
  };
  let s_box: Arc<Segment> = Arc::new(new_seg);
  parser.segments.push(s_box.clone());
  if parser.in_transaction() {
    match &mut parser.current_transaction {
      None => (),
      Some(ct) => {
        let mut curr = ct.borrow_mut();
        curr.segments.push(s_box.clone());
      }
    }
    match &mut parser.current_functional_group {
      None => (),
      Some(fg) => {
        let mut curr = fg.borrow_mut();
        curr.segments.push(s_box.clone());
      }
    }
    match &mut parser.current_interchange {
      None => (),
      Some(ci) => {
        let mut curr = ci.borrow_mut();
        curr.segments.push(s_box);
      }
    }
  } else if parser.in_functional_group() {
    match &mut parser.current_functional_group {
      None => (),
      Some(fg) => {
        let mut curr = fg.borrow_mut();
        curr.segments.push(s_box.clone());
      }
    }
    match &mut parser.current_interchange {
      None => (),
      Some(ci) => {
        let mut curr = ci.borrow_mut();
        curr.segments.push(s_box);
      }
    }
  } else if parser.in_interchange() {
    match &mut parser.current_interchange {
      None => (),
      Some(ci) => {
        let mut curr = ci.borrow_mut();
        curr.segments.push(s_box);
      }
    }
  }
}

#[cfg(test)]
mod test {
  use super::DefaultParser;
  use crate::edi_parsers::create_edi_streamer;
  use crate::edi_parsers::execute_streaming_parser;
  use std::io::Cursor;

  #[test]
  fn simple_interchange_test() {
    let mut dp = DefaultParser::new();
    let mut ioish = Cursor::new("ISA*00*TSI       *01*92511930  *01*ME             *12*BRADLEY        *970815*1732*U*00201*000000050*0*T*>~\n".as_bytes());
    let edis = &mut create_edi_streamer(&mut ioish);
    match edis {
      Ok(pi) => {
        execute_streaming_parser(pi, &mut dp);
        assert_eq!("ISA", String::from_utf8_lossy(dp.interchanges[0].segments[0].tag.as_slice()));
      },
      Err(_e) => panic!("FAILED TO CREATE PARSER")
    }
  }

  #[test]
  fn fg_and_transaction_simple_test() {
    let raw = "\
ISA*00*TSI       *01*92511930  *01*ME             *12*BRADLEY        *970815*1732*U*00201*000000050*0*T*>~
GS**~
ST~
SE~
GE~
IEA~
    ";
    let mut dp = DefaultParser::new();
    let mut ioish = Cursor::new(raw.as_bytes());
    let edis = &mut create_edi_streamer(&mut ioish);
    match edis {
      Ok(pi) => {
        execute_streaming_parser(pi, &mut dp);
        assert_eq!("ISA", String::from_utf8_lossy(dp.interchanges[0].segments[0].tag.as_slice()));
        assert_eq!(dp.interchanges[0].functional_groups.len(), 1);
        assert_eq!(dp.interchanges[0].functional_groups[0].transactions.len(), 1);
      },
      Err(_e) => panic!("FAILED TO CREATE PARSER")
    }
  }

  #[test]
  fn fg_and_transaction_weird_test() {
    let raw = "\
ISA*00*TSI       *01*92511930  *01*ME             *12*BRADLEY        *970815*1732*U*00201*000000050*0*T*>~
ISA~
GS~
ISA~
GS~
ST~
ISA~
GS~
ST~
GE~
ISA~
GS~
ST~
GS~
ISA~
GS~
ST~
ST~
GE~
IEA~
";
    let mut dp = DefaultParser::new();
    let mut ioish = Cursor::new(raw.as_bytes());
    let edis = &mut create_edi_streamer(&mut ioish);
    match edis {
      Ok(pi) => {
        execute_streaming_parser(pi, &mut dp);
        assert_eq!("ISA", String::from_utf8_lossy(dp.interchanges[0].segments[0].tag.as_slice()));
        assert_eq!(dp.interchanges.len(), 6);
        assert_eq!(dp.interchanges[4].functional_groups.len(), 2);
        assert_eq!(dp.interchanges[5].functional_groups[0].transactions.len(), 2);
      },
      Err(_e) => panic!("FAILED TO CREATE PARSER")
    }
  }
}