use actix_web::{web, HttpResponse, Responder};
use golem_base_sdk::entity::{Create, Update};
use golem_base_sdk::{Address, GolemBaseClient, Hash};

pub async fn create_entity(
    client: web::Data<GolemBaseClient>,
    item: web::Json<Create>,
) -> impl Responder {
    match client.create_entities(vec![item.into_inner()]).await {
        Ok(receipts) => HttpResponse::Created().json(receipts),
        Err(e) => HttpResponse::InternalServerError().body(format!("Error creating entity: {}", e)),
    }
}

pub async fn get_entities(
    client: web::Data<GolemBaseClient>,
    owner_address: web::Path<Address>,
) -> impl Responder {
    match client
        .get_entities_of_owner(owner_address.into_inner())
        .await
    {
        Ok(entities) => HttpResponse::Ok().json(entities),
        Err(e) => {
            HttpResponse::InternalServerError().body(format!("Error fetching entities: {}", e))
        }
    }
}

pub async fn update_entity(
    client: web::Data<GolemBaseClient>,
    item: web::Json<Update>,
) -> impl Responder {
    match client.update_entities(vec![item.into_inner()]).await {
        Ok(_) => HttpResponse::Ok().body("Entity updated successfully"),
        Err(e) => HttpResponse::InternalServerError().body(format!("Error updating entity: {}", e)),
    }
}

pub async fn delete_entity(
    client: web::Data<GolemBaseClient>,
    entity_key: web::Path<String>,
) -> impl Responder {
    let key: Hash = entity_key
        .into_inner()
        .parse()
        .expect("Invalid entity key format");
    match client.delete_entities(vec![key]).await {
        Ok(_) => HttpResponse::Ok().body("Entity deleted successfully"),
        Err(e) => HttpResponse::InternalServerError().body(format!("Error deleting entity: {}", e)),
    }
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("")
            .route("/entities", web::post().to(create_entity))
            .route("/entities/{id}", web::get().to(get_entities))
            .route("/entities/{id}", web::put().to(update_entity))
            .route("/entities/{id}", web::delete().to(delete_entity)),
    );
}
