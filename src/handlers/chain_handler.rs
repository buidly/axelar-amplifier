use crate::event_processor::EventHandler;
use crate::event_sub::Event;
use async_trait::async_trait;
use error_stack::{Result, ResultExt};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChainHandlerError {
    #[error("one of the chained handlers failed handling event")]
    ChainError,
}

pub struct ChainHandler<H1, H2>
where
    H1: EventHandler,
    H2: EventHandler,
{
    handler_1: H1,
    handler_2: H2,
}

impl<H1, H2> ChainHandler<H1, H2>
where
    H1: EventHandler,
    H2: EventHandler,
{
    pub fn new(handler_1: H1, handler_2: H2) -> Self {
        Self { handler_1, handler_2 }
    }
}

#[async_trait]
impl<H1, H2> EventHandler for ChainHandler<H1, H2>
where
    H1: EventHandler,
    H2: EventHandler,
{
    type Err = ChainHandlerError;

    async fn handle(&self, event: &Event) -> Result<(), ChainHandlerError> {
        self.handler_1
            .handle(event)
            .await
            .change_context(ChainHandlerError::ChainError)?;
        self.handler_2
            .handle(event)
            .await
            .change_context(ChainHandlerError::ChainError)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::event_processor::{self, EventHandler};
    use crate::event_sub;
    use async_trait::async_trait;
    use error_stack::{IntoReport, Result};
    use mockall::{mock, predicate};
    use tendermint::block;
    use thiserror::Error;

    #[tokio::test]
    async fn should_chain_handlers() {
        let height: block::Height = (10_u32).into();

        let mut handler_1 = MockEventHandler::new();
        handler_1
            .expect_handle()
            .once()
            .with(predicate::eq(event_sub::Event::BlockEnd(height)))
            .returning(|_| Ok(()));
        let mut handler_2 = MockEventHandler::new();
        handler_2
            .expect_handle()
            .once()
            .with(predicate::eq(event_sub::Event::BlockEnd(height)))
            .returning(|_| Ok(()));

        assert!(handler_1
            .chain(handler_2)
            .handle(&event_sub::Event::BlockEnd(height))
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn should_fail_if_the_first_handler_fails() {
        let height: block::Height = (10_u32).into();

        let mut handler_1 = MockEventHandler::new();
        handler_1
            .expect_handle()
            .once()
            .with(predicate::eq(event_sub::Event::BlockEnd(height)))
            .returning(|_| Err(EventHandlerError::Unknown).into_report());

        assert!(handler_1
            .chain(MockEventHandler::new())
            .handle(&event_sub::Event::BlockEnd(height))
            .await
            .is_err());
    }

    #[tokio::test]
    async fn should_fail_if_the_second_handler_fails() {
        let height: block::Height = (10_u32).into();

        let mut handler_1 = MockEventHandler::new();
        handler_1
            .expect_handle()
            .once()
            .with(predicate::eq(event_sub::Event::BlockEnd(height)))
            .returning(|_| Ok(()));
        let mut handler_2 = MockEventHandler::new();
        handler_2
            .expect_handle()
            .once()
            .with(predicate::eq(event_sub::Event::BlockEnd(height)))
            .returning(|_| Err(EventHandlerError::Unknown).into_report());

        assert!(handler_1
            .chain(handler_2)
            .handle(&event_sub::Event::BlockEnd(height))
            .await
            .is_err());
    }

    #[derive(Error, Debug)]
    pub enum EventHandlerError {
        #[error("unknown")]
        Unknown,
    }

    mock! {
            EventHandler{}

            #[async_trait]
            impl event_processor::EventHandler for EventHandler {
                type Err = EventHandlerError;

                async fn handle(&self, event: &event_sub::Event) -> Result<(), EventHandlerError>;
            }
    }
}
