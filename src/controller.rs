pub trait KeyController {
    fn receive(&mut self, key: crate::Key);
}

pub trait ActionController {
    type Action: Clone;
    fn receive(&mut self, action: Self::Action) -> bool;
}

struct ComposedActionController<A, B> {
    a: A,
    b: B,
}

impl <A,B> ActionController for ComposedActionController<A,B>
where A: ActionController, B: ActionController<Action = A::Action> {
    type Action = A::Action;
    fn receive(&mut self, action: Self::Action) -> bool {
        let res_a = self.a.receive(action.clone());
        if res_a {
            res_a
        } else {
            self.b.receive(action)
        }
    }
}

fn compose<A,B,C>(a: A, b: B) -> C where A: ActionController, B: ActionController, C: ActionController {
    unimplemented!()
}

