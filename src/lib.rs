use std::{
  cell::RefCell,
  collections::{btree_map::Entry, BTreeMap, BTreeSet},
  rc::Rc,
};
use thiserror::Error;

pub type DepId = (u64, usize);

#[derive(Clone, Debug, Error)]
pub enum DepTreeBuilderError {
  #[error("unit `{0:?}` depends on itself")]
  SelfDependency(DepId),
  #[error("unit `{0:?}` recurses when depending on `{1:?}`, `{2}`")]
  CircularDependency(DepId, DepId, String),
}

pub type DepTreeBuilderResult<T> = Result<T, DepTreeBuilderError>;

#[derive(Clone, Debug, Default)]
pub struct DepTreeBuilder {
  inner: Rc<RefCell<Box<BTreeMap<DepId, Vec<DepId>>>>>,
}

impl DepTreeBuilder {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn with_dep(&mut self, id: DepId, deps: Vec<DepId>) -> Self {
    let mut inner_lock = self.inner.try_borrow_mut().unwrap();
    match inner_lock.entry(id) {
      Entry::Vacant(entry) => {
        entry.insert(deps);
      }
      Entry::Occupied(mut entry) => {
        entry.get_mut().extend(deps);
      }
    }
    self.clone()
  }
  
  pub fn build(self) -> DepTreeBuilderResult<Box<DepTree>> {
    let inner = self.inner.try_borrow().unwrap();
    let (mut visited, mut resolved): (
      Vec<DepId>,
      BTreeMap<DepId, Vec<DepId>>,
    ) = (
      Vec::new(),
      BTreeMap::new(),
    );
    for (unit, deps) in inner.clone().into_iter() {
      if deps.contains(&unit) {
        return Err(DepTreeBuilderError::SelfDependency(unit));
      }
      let mut stack = Vec::new();
      if self.has_circular_dependency(unit, &inner, &mut visited, &mut stack) {
        return Err(DepTreeBuilderError::CircularDependency(
          *stack.first().unwrap(),
          *stack.last().unwrap(),
          stack
            .iter()
            .map(|(id, version)| format!("({id}, {version})"))
            .collect::<Vec<_>>()
            .join(" -> ")
        ));
      }
      resolved.insert(unit, deps);
    }
    Ok(Box::new(DepTree::new(Rc::new(resolved))))
  }

  fn has_circular_dependency(
    &self,
    unit: DepId,
    tree: &BTreeMap<DepId, Vec<DepId>>,
    visited: &mut Vec<DepId>,
    stack: &mut Vec<DepId>,
  ) -> bool {
    if visited.contains(&unit) {
      return false;
    }
    if stack.contains(&unit) {
      return true;
    }
    stack.push(unit);
    if let Some(deps) = tree.get(&unit) {
      for &dep in deps {
        if self.has_circular_dependency(dep, tree, visited, stack) {
          return true;
        }
      }
    }
    stack.pop();
    visited.push(unit);
    false
  }
}

#[derive(Clone, Debug, Default)]
pub struct DepTree {
  inner: Rc<BTreeMap<DepId, Vec<DepId>>>,
}

impl DepTree {
  pub fn new(inner: Rc<BTreeMap<DepId, Vec<DepId>>>) -> Self {
    Self { inner }
  }
  
  pub fn most_dependencies(&self) -> Vec<(DepId, usize)> {
    let mut dependency_counts = self.inner.keys().map(|id| {
      let count = self.count_dependencies(id, &mut BTreeSet::new());
      (*id, count)
    }).collect::<Vec<_>>();

    dependency_counts.sort_by(|a, b| b.1.cmp(&a.1));
    dependency_counts
  }

  pub fn most_dependents(&self) -> Vec<(DepId, usize)> {
    let mut dependent_counts = self.calculate_dependents();
    dependent_counts.sort_by(|a, b| b.1.cmp(&a.1));
    dependent_counts
  }

  pub fn least_dependencies(&self) -> Vec<(DepId, usize)> {
    let mut dependency_counts = self.inner.keys().map(|id| {
      let count = self.count_dependencies(id, &mut BTreeSet::new());
      (*id, count)
    }).collect::<Vec<_>>();

    dependency_counts.sort_by(|a, b| a.1.cmp(&b.1));
    dependency_counts
  }

  pub fn least_dependents(&self) -> Vec<(DepId, usize)> {
    let mut dependent_counts = self.calculate_dependents();
    dependent_counts.sort_by(|a, b| a.1.cmp(&b.1));
    dependent_counts
  }

  pub fn dependencies_of(&self, unit: DepId) -> Vec<DepId> {
    let mut visited = BTreeSet::new();
    let mut dependencies = Vec::new();
    self.collect_dependencies(&unit, &mut visited, &mut dependencies);
    dependencies
  }

  pub fn dependents_of(&self, unit: DepId) -> Vec<DepId> {
    self.inner
      .iter()
      .filter_map(|(&key, deps)| {
        if deps.contains(&unit) {
          Some(key)
        } else {
          None
        }
      })
      .collect()
  }

  fn count_dependencies(&self, id: &DepId, visited: &mut BTreeSet<DepId>) -> usize {
    if !visited.insert(*id) {
      return 0;
    }
    self.inner
      .get(id)
      .map(|deps| {
        deps
          .iter()
          .map(|dep| 1 + self.count_dependencies(dep, visited))
          .sum()
      })
      .unwrap_or(0)
  }

  fn collect_dependencies(&self, id: &DepId, visited: &mut BTreeSet<DepId>, dependencies: &mut Vec<DepId>) {
    if !visited.insert(*id) {
      return;
    }
    if let Some(deps) = self.inner.get(id) {
      for dep in deps {
        dependencies.push(*dep);
        self.collect_dependencies(dep, visited, dependencies);
      }
    }
  }

  fn calculate_dependents(&self) -> Vec<(DepId, usize)> {
    let mut dependent_map: BTreeMap<DepId, usize> = BTreeMap::new();
    
    for (&key, deps) in self.inner.iter() {
      for &dep in deps {
        *dependent_map.entry(dep).or_insert(0) += 1;
      }
      dependent_map.entry(key).or_insert(0);
    }

    dependent_map.into_iter().collect()
  }
}