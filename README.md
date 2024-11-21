Dependency tree structure for building dependency requirements

Example:
```rs
use dep_tree::DepTreeBuilder;

fn main() {
  let deps = DepTreeBuilder::new()
    .with_dep((1, 1), vec![(1, 2), (1, 3)])
    .with_dep((1, 2), vec![])
    .with_dep((1, 3), vec![(1, 3)]) // Self-dependency
    .with_dep((2, 0), vec![(2, 1)]) 
    .with_dep((2, 1), vec![(2, 2)]) // Circular dependency
    .with_dep((2, 2), vec![(2, 1)])
    .with_dep((2, 3), vec![])
    .build();
  println!("{}", deps.unwrap_or_else(|e| panic!("{}", e.to_string())));
}
```