use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use tokio::sync::Mutex as AsyncMutex;

use crate::{
    proxy::Toxics,
    toxic::{Toxic, ToxicKind},
};

#[derive(Debug, PartialEq)]
pub enum ToxicState {
    LimitData { bytes_transmitted: usize },
}

impl ToxicState {
    pub fn for_toxic_kind(kind: &ToxicKind) -> Option<ToxicState> {
        match kind {
            ToxicKind::LimitData { .. } => Some(ToxicState::LimitData {
                bytes_transmitted: 0,
            }),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub(crate) struct ToxicStateHolder {
    inner: Mutex<HashMap<String, Arc<AsyncMutex<ToxicState>>>>,
}

impl ToxicStateHolder {
    pub(crate) fn for_toxics(toxics: &Toxics) -> Option<Arc<ToxicStateHolder>> {
        let toxics_pair: [&[Toxic]; 2] = [&toxics.upstream, &toxics.downstream];
        let stateful_toxics: Vec<&Toxic> = toxics_pair
            .iter()
            .flat_map(|direction_toxics| {
                direction_toxics
                    .iter()
                    .filter(|toxic| toxic.kind.is_stateful())
            })
            .collect();

        if stateful_toxics.is_empty() {
            None
        } else {
            let mut state_map: HashMap<String, Arc<AsyncMutex<ToxicState>>> = HashMap::new();

            for toxic in stateful_toxics {
                if let Some(initial_toxic_state) = ToxicState::for_toxic_kind(&toxic.kind) {
                    state_map.insert(
                        toxic.name.to_owned(),
                        Arc::new(AsyncMutex::new(initial_toxic_state)),
                    );
                }
            }
            Some(Arc::new(ToxicStateHolder {
                inner: Mutex::new(state_map),
            }))
        }
    }

    pub(crate) fn get_state_for_toxic(
        &self,
        toxic_name: &str,
    ) -> Option<Arc<AsyncMutex<ToxicState>>> {
        let inner = self.inner.lock().expect("ToxicStateHolder lock poisoned");
        inner.get(toxic_name).map(|toxic| Arc::clone(toxic))
    }
}
