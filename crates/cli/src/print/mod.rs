mod cloud_print;
mod colored_print;
mod interactive_print;
mod json_print;

use crate::lang::SgLang;
use ast_grep_config::{Fixer, RuleConfig};
use ast_grep_core::{tree_sitter::StrDoc, Matcher, NodeMatch as SgNodeMatch};

use anyhow::Result;
use clap::ValueEnum;

use std::borrow::Cow;
use std::path::Path;

pub use cloud_print::{CloudPrinter, Platform};
pub use codespan_reporting::files::SimpleFile;
use codespan_reporting::term::termcolor::ColorChoice;
pub use colored_print::{ColoredPrinter, Heading, ReportStyle};
pub use interactive_print::InteractivePrinter;
pub use json_print::{JSONPrinter, JsonStyle};

type NodeMatch<'a> = SgNodeMatch<'a, StrDoc<SgLang>>;

/// A trait to process nodeMatches to diff/match output
/// it must be Send + 'static to be shared in worker thread
pub trait PrintProcessor<Output>: Send + Sync + 'static {
  fn print_rule(
    &self,
    matches: Vec<NodeMatch>,
    file: SimpleFile<Cow<str>, &str>,
    rule: &RuleConfig<SgLang>,
  ) -> Result<Output>;
  fn print_matches(&self, matches: Vec<NodeMatch>, path: &Path) -> Result<Output>;
  fn print_diffs(&self, diffs: Vec<Diff>, path: &Path) -> Result<Output>;
  fn print_rule_diffs(
    &self,
    diffs: Vec<(Diff, &RuleConfig<SgLang>)>,
    path: &Path,
  ) -> Result<Output>;
}

pub trait Printer {
  // processed item must be sent to printer thread
  type Processed: Send + 'static;
  type Processor: PrintProcessor<Self::Processed>;

  fn get_processor(&self) -> Self::Processor;
  /// Runs processed output from processor. This runs multiple times.
  fn process(&mut self, processed: Self::Processed) -> Result<()>;

  /// Run before all printing. One CLI will run this exactly once.
  #[inline]
  fn before_print(&mut self) -> Result<()> {
    Ok(())
  }
  /// Run after all printing. One CLI will run this exactly once.
  #[inline]
  fn after_print(&mut self) -> Result<()> {
    Ok(())
  }
}

#[derive(Clone)]
pub struct AdditionalFix {
  pub replacement: String,
  pub range: std::ops::Range<usize>,
  pub title: Option<String>,
}

#[derive(Clone)]
pub struct Diff<'n> {
  /// the matched node
  pub node_match: NodeMatch<'n>,
  /// string content for the replacement
  pub replacement: String,
  pub range: std::ops::Range<usize>,
  pub title: Option<String>,
  pub additional_fixes: Option<Box<[AdditionalFix]>>,
}

impl<'n> Diff<'n> {
  pub fn generate(node_match: NodeMatch<'n>, matcher: &impl Matcher, rewrite: &Fixer) -> Self {
    let edit = node_match.make_edit(matcher, rewrite);
    let replacement = String::from_utf8(edit.inserted_text).unwrap();
    Self {
      node_match,
      replacement,
      range: edit.position..edit.position + edit.deleted_length,
      additional_fixes: None,
      title: rewrite.title().map(|t| t.to_string()),
    }
  }

  pub fn multiple(
    node_match: NodeMatch<'n>,
    matcher: &impl Matcher,
    fixers: &[Fixer],
  ) -> Option<Self> {
    let fixer = fixers.first()?;
    let mut ret = Self::generate(node_match.clone(), matcher, fixer);
    // no additional fixes
    if fixers.len() == 1 {
      return Some(ret);
    }
    let additional = fixers
      .iter()
      .skip(1)
      .map(|f| {
        let edit = node_match.make_edit(matcher, f);
        AdditionalFix {
          replacement: String::from_utf8(edit.inserted_text).unwrap(),
          range: edit.position..edit.position + edit.deleted_length,
          title: f.title().map(|t| t.to_string()),
        }
      })
      .collect::<Vec<_>>()
      .into_boxed_slice();
    ret.additional_fixes = Some(additional);
    Some(ret)
  }

  pub fn into_list(mut self) -> Vec<Self> {
    let node_match = self.node_match.clone();
    let additional_fixes = self.additional_fixes.take();
    let mut ret = vec![self];
    ret.extend(additional_fixes.into_iter().flatten().map(|f| Self {
      node_match: node_match.clone(),
      replacement: f.replacement,
      range: f.range,
      additional_fixes: None,
      title: f.title,
    }));
    ret
  }

  /// Returns the root doc source code
  /// N.B. this can be different from node.text() because
  /// tree-sitter's root Node may not start at the begining
  pub fn get_root_text(&self) -> &'n str {
    self.node_match.root().get_text()
  }
}

#[derive(ValueEnum, Clone, Copy)]
pub enum ColorArg {
  /// Try to use colors, but don't force the issue. If the output is piped to another program,
  /// or the console isn't available on Windows, or if TERM=dumb, or if `NO_COLOR` is defined,
  /// for example, then don't use colors.
  Auto,
  /// Try very hard to emit colors. This includes emitting ANSI colors
  /// on Windows if the console API is unavailable (not implemented yet).
  Always,
  /// Ansi is like Always, except it never tries to use anything other
  /// than emitting ANSI color codes.
  Ansi,
  /// Never emit colors.
  Never,
}

impl ColorArg {
  pub fn should_use_color(self) -> bool {
    use colored_print::should_use_color;
    should_use_color(&self.into())
  }
}

impl From<ColorArg> for ColorChoice {
  fn from(arg: ColorArg) -> ColorChoice {
    use ColorArg::*;
    match arg {
      Auto => {
        if atty::is(atty::Stream::Stdout) {
          ColorChoice::Auto
        } else {
          ColorChoice::Never
        }
      }
      Always => ColorChoice::Always,
      Ansi => ColorChoice::AlwaysAnsi,
      Never => ColorChoice::Never,
    }
  }
}
