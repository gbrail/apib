use httptarget::Target;
use reqwest::StatusCode;

#[tokio::test]
async fn test_get() {
    let svr = Target::new(0, true).await.expect("Error starting server");
    let url = format!("http://{}/hello", svr.address());
    let response = reqwest::get(url).await.expect("Error getting result");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.text().await.expect("Error getting body");
    assert_eq!(body, "Hello, World!");
}

#[tokio::test]
async fn test_not_found() {
    let svr = Target::new(0, true).await.expect("Error starting server");
    let url = format!("http://{}/NOTFOUND", svr.address());
    let response = reqwest::get(url).await.expect("Error getting result");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    response.bytes().await.expect("Error getting body");
}

#[tokio::test]
async fn test_wrong_method() {
    let svr = Target::new(0, true).await.expect("Error starting server");
    let url = format!("http://{}/hello", svr.address());
    let client = reqwest::Client::new();
    let response = client
        .post(url)
        .body("")
        .send()
        .await
        .expect("Error getting result");
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    response.bytes().await.expect("Error getting body");
}
