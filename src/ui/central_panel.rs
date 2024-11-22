use std::sync::{Arc, Mutex};

use super::DiscidiumData;

use widget::node::BoxedUiNode;
use zng::prelude::*;

pub fn central_panel(data: Arc<Mutex<DiscidiumData>>) -> BoxedUiNode {
    Box::new(Text!(formatx!("{:?}", data.lock().unwrap().state)))
}
