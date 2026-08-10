#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Runtime {
  InternetExplorer,
  Edge,
  Firefox,
  Chrome,
  Safari,
  Opera,
  Ios,
  OperaMini,
  Android,
  Blackberry,
  OperaMobile,
  ChromeAndroid,
  FirefoxAndroid,
  InternetExplorerMobile,
  UcAndroid,
  SamsungInternet,
  QqAndroid,
  Baidu,
  KaiOS,
  Node,
}

impl Runtime {
  pub(crate) const fn browserslist_name(self) -> &'static str {
    match self {
      Self::InternetExplorer => "ie",
      Self::Edge => "edge",
      Self::Firefox => "firefox",
      Self::Chrome => "chrome",
      Self::Safari => "safari",
      Self::Opera => "opera",
      Self::Ios => "ios_saf",
      Self::OperaMini => "op_mini",
      Self::Android => "android",
      Self::Blackberry => "bb",
      Self::OperaMobile => "op_mob",
      Self::ChromeAndroid => "and_chr",
      Self::FirefoxAndroid => "and_ff",
      Self::InternetExplorerMobile => "ie_mob",
      Self::UcAndroid => "and_uc",
      Self::SamsungInternet => "samsung",
      Self::QqAndroid => "and_qq",
      Self::Baidu => "baidu",
      Self::KaiOS => "kaios",
      Self::Node => "node",
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn exposes_the_canonical_browserslist_name() {
    assert_eq!(Runtime::Chrome.browserslist_name(), "chrome");
    assert_eq!(Runtime::ChromeAndroid.browserslist_name(), "and_chr");
    assert_eq!(Runtime::Ios.browserslist_name(), "ios_saf");
    assert_eq!(Runtime::Node.browserslist_name(), "node");
  }
}
