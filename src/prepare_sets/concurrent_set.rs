use std::sync::Arc;

use futures::{
    future::{join, ok, Ready},
    Future, FutureExt, TryFutureExt,
};

use crate::{
    fn_prepare, prepared_effect::CombineEffects, EffectsCollector, ExtensionEffect, FnPrepare,
    GracefulEffect, Prepare, PrepareError, PrepareHandler, PreparedEffect, RouteEffect,
    ServerEffect,
};

/// apply all prepare task concurrently
pub struct ConcurrentPrepareSet<C, PFut> {
    prepare_fut: PFut,
    configure: Arc<C>,
}

impl<C, PFut, E> ConcurrentPrepareSet<C, PFut>
where
    PFut: Future<Output = Result<E, PrepareError>>,
    E: PreparedEffect,
{
    pub fn to_prepared_effect(self) -> PFut {
        self.prepare_fut
    }
}

impl<C, PFut, R, S, G, E> ConcurrentPrepareSet<C, PFut>
where
    PFut: Future<Output = Result<EffectsCollector<R, G, E, S>, PrepareError>>,
    R: RouteEffect,
    G: GracefulEffect,
    S: ServerEffect,
    E: ExtensionEffect,
    C: 'static,
{
    pub fn join<P: Prepare<C>>(
        self,
        prepare: P,
    ) -> ConcurrentPrepareSet<
        C,
        impl Future<Output = Result<CombineEffects<R, G, E, S, P::Effect>, PrepareError>>,
    > {
        let configure = Arc::clone(&self.configure);
        let prepare_fut = join(
            self.prepare_fut,
            prepare
                .prepare(configure)
                .map_err(PrepareError::to_prepare_error::<P, _>),
        )
        .map(|(l, r)| Ok(l?.with_effect(r?)));

        ConcurrentPrepareSet {
            prepare_fut,
            configure: self.configure,
        }
    }

    pub fn join_fn<F, Args>(
        self,
        func: F,
    ) -> ConcurrentPrepareSet<
        C,
        impl Future<
            Output = Result<
                CombineEffects<R, G, E, S, <FnPrepare<C, Args, F> as Prepare<C>>::Effect>,
                PrepareError,
            >,
        >,
    >
    where
        F: PrepareHandler<Args, C>,
    {
        self.join(fn_prepare(func))
    }
}

impl<C: 'static> ConcurrentPrepareSet<C, Ready<Result<EffectsCollector, PrepareError>>> {
    pub(crate) fn new(configure: Arc<C>) -> Self {
        Self {
            prepare_fut: ok(EffectsCollector::new()),
            configure,
        }
    }
}
