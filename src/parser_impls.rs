use crate::edi_parsers::StreamParser;
use crate::edi_segments::Segment;
use std::rc::Rc;

#[derive(PartialEq)]
enum ParserState {
  Nothing,
  InInterchange,
  InFunctionalGroup,
  InTransaction
}

struct Interchange {
  segments: Vec<Rc<Segment>>
}

struct FunctionalGroup {
  segments: Vec<Rc<Segment>>
}

struct Transaction {
  segments: Vec<Rc<Segment>>
}

pub struct DefaultParser<'a> {
  state: ParserState,
  current_interchange: Option<&'a mut Interchange>,
  current_functional_group: Option<&'a mut FunctionalGroup>,
  current_transaction: Option<&'a mut Transaction>,
  segments: Vec<Rc<Segment>>
}

impl<'a> DefaultParser<'a> {
  pub fn new() -> Self {
    DefaultParser {
      state: ParserState::Nothing,
      current_interchange: None,
      current_functional_group: None,
      current_transaction: None,
      segments: Vec::new()
    }
  }
}

impl<'a> StreamParser for DefaultParser<'a> {
  fn transaction_start(&mut self, segment: &Segment) {
    self.state = ParserState::InTransaction;
  }

  fn transaction_end(&mut self, segment: Option<&Segment>) {
    self.state = ParserState::InFunctionalGroup;
  }

  fn functional_group_start(&mut self, segment: &Segment) {
    self.state = ParserState::InFunctionalGroup;
  }

  fn functional_group_end(&mut self, segment: Option<&Segment>) {
    self.state = ParserState::InInterchange;
  }

  fn interchange_start(&mut self, segment: &Segment) {
    self.state = ParserState::InInterchange;
  }

  fn interchange_end(&mut self, segment: Option<&Segment>) {
    self.state = ParserState::Nothing;
  }

  fn error(&mut self, error: std::io::Error) {

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
  let s_box: Rc<Segment> = Rc::new(new_seg);
  parser.segments.push(s_box.clone());
  if parser.in_transaction() {
    match &mut parser.current_transaction {
      None => (),
      Some(ct) => {
        ct.segments.push(s_box.clone());
      }
    }
    match &mut parser.current_functional_group {
      None => (),
      Some(fg) => {
        fg.segments.push(s_box.clone());
      }
    }
    match &mut parser.current_interchange {
      None => (),
      Some(ci) => {
        ci.segments.push(s_box.clone());
      }
    }
  } else if parser.in_functional_group() {
    match &mut parser.current_functional_group {
      None => (),
      Some(fg) => {
        fg.segments.push(s_box.clone());
      }
    }
    match &mut parser.current_interchange {
      None => (),
      Some(ci) => {
        ci.segments.push(s_box.clone());
      }
    }
  } else if parser.in_interchange() {
  match &mut parser.current_interchange {
    None => (),
    Some(ci) => {
      ci.segments.push(s_box.clone());
    }
  }
}

}