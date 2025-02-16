pub mod fetch {
    use axum::{extract::Path, http::StatusCode};
    use axum_macros::debug_handler;
    use std::fs::File;
    use thiserror::Error;
    use tracing::{event, Level};
    use url::Url;

    #[derive(Error, Debug)]
    pub enum ImageFetchError {
        #[error("failed to create url")]
        InvalidUrl(#[from] url::ParseError),

        #[error("failed to fetch")]
        FailedToFetch(#[from] reqwest::Error),

        #[error("failed to save image")]
        FailedToSave(#[from] std::io::Error),
    }

    struct Size {
        pub x: u16,
        pub y: u16,
    }

    impl Size {
        pub fn to_url(&self) -> Result<Url, ImageFetchError> {
            let raw = format!("https://picsum.photos/{}/{}", &self.x, &self.y);
            let url = Url::parse(&raw)?;
            Ok(url)
        }

        pub fn to_file_name(&self) -> String {
            let file_name = format!("image-{}-{}.png", &self.x, &self.y);
            file_name
        }
    }

    #[debug_handler]
    pub async fn handle(path: Path<(u16, u16)>) -> axum::response::Result<(), StatusCode> {
        match handle_inner(path).await {
            Ok(()) => Ok(()),
            Err(e) => {
                let message = e.to_string();
                event!(Level::ERROR, "{}", &message);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }

    async fn handle_inner(Path((x, y)): Path<(u16, u16)>) -> Result<(), ImageFetchError> {
        let size = Size { x, y };
        let resp = send_get(&size).await?;
        save(&size, resp).await?;
        Ok(())
    }

    async fn send_get(size: &Size) -> Result<reqwest::Response, ImageFetchError> {
        let url = size.to_url()?;
        let response = reqwest::get(url).await?;
        Ok(response)
    }

    async fn save(size: &Size, resp: reqwest::Response) -> Result<(), ImageFetchError> {
        let bytes = resp.bytes().await?;
        let file_name = size.to_file_name();
        let mut out_file = File::create(file_name)?;
        std::io::copy(&mut bytes.as_ref(), &mut out_file)?;
        Ok(())
    }
}
