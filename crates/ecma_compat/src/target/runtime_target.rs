use super::{Runtime, RuntimeRelease};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RuntimeTarget {
  runtime: Runtime,
  release: RuntimeRelease,
}

impl RuntimeTarget {
  pub const fn new(runtime: Runtime, release: RuntimeRelease) -> Self {
    Self { runtime, release }
  }

  pub const fn runtime(self) -> Runtime {
    self.runtime
  }

  pub const fn release(self) -> RuntimeRelease {
    self.release
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::target::Version;

  #[test]
  fn creates_a_runtime_target_from_validated_values() {
    let target = RuntimeTarget::new(
      Runtime::Safari,
      RuntimeRelease::Exact(Version::new(13, 1, 0, 0)),
    );

    assert_eq!(target.runtime(), Runtime::Safari);
    assert_eq!(
      target.release(),
      RuntimeRelease::Exact(Version::new(13, 1, 0, 0)),
    );
  }
}
