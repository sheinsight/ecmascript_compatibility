use browserslist::{Opts, resolve};

use crate::error::TargetQueryError;

// #[cfg(test)]
// mod tests {
//   use super::*;

//   #[test]
//   fn resolves_chrome_79() {
//     let query = TargetQuery::new(["chrome 79"]).unwrap();

//     let targets = TargetResolver.resolve(&query).unwrap();

//     assert_eq!(
//       targets,
//       vec![RuntimeTarget {
//         browser: "chrome".to_owned(),
//         version: 79,
//       },],
//     );
//   }

//   #[test]
//   fn resolves_multiple_chrome_versions() {
//     let query = TargetQuery::new(["chrome 79", "chrome 80"]).unwrap();

//     let targets = TargetResolver.resolve(&query).unwrap();

//     assert_eq!(
//       targets,
//       vec![
//         RuntimeTarget {
//           browser: "chrome".to_owned(),
//           version: 79,
//         },
//         RuntimeTarget {
//           browser: "chrome".to_owned(),
//           version: 80,
//         },
//       ],
//     );
//   }

//   #[test]
//   fn rejects_invalid_query() {
//     let query = TargetQuery::new(["invalid-browser 1"]).unwrap();

//     let result = TargetResolver.resolve(&query);

//     assert!(result.is_err());
//   }
// }
