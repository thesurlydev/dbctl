//! Schema templates for common database patterns

use super::models::{
    ColumnModel, ConstraintModel, FkAction, IndexModel, SchemaModel, TableModel,
};
use eframe::egui;
use std::collections::HashMap;

/// Template category for organization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateCategory {
    Comprehensive,
    Starter,
}

impl TemplateCategory {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Comprehensive => "Comprehensive Patterns",
            Self::Starter => "Minimal Starters",
        }
    }
}

/// A schema template definition
#[derive(Debug, Clone)]
pub struct SchemaTemplate {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub category: TemplateCategory,
    pub tags: Vec<&'static str>,
}

impl SchemaTemplate {
    /// Generate the actual SchemaModel for this template
    pub fn generate(&self) -> SchemaModel {
        match self.id {
            "ecommerce" => generate_ecommerce_template(),
            "blog_cms" => generate_blog_cms_template(),
            "saas_auth" => generate_saas_auth_template(),
            "project_management" => generate_project_management_template(),
            "social_media" => generate_social_media_template(),
            "users_starter" => generate_users_starter_template(),
            "basic_crud" => generate_basic_crud_template(),
            "audit_logging" => generate_audit_logging_template(),
            _ => SchemaModel::default(),
        }
    }

    /// Get the number of tables in this template
    pub fn table_count(&self) -> usize {
        match self.id {
            "ecommerce" => 6,
            "blog_cms" => 6,
            "saas_auth" => 6,
            "project_management" => 5,
            "social_media" => 5,
            "users_starter" => 1,
            "basic_crud" => 1,
            "audit_logging" => 1,
            _ => 0,
        }
    }
}

/// Get all available templates
pub fn get_all_templates() -> Vec<SchemaTemplate> {
    vec![
        // Comprehensive Patterns
        SchemaTemplate {
            id: "ecommerce",
            name: "E-commerce",
            description: "Products, categories, orders, customers, payments, inventory",
            category: TemplateCategory::Comprehensive,
            tags: vec!["shop", "store", "products", "orders"],
        },
        SchemaTemplate {
            id: "blog_cms",
            name: "Blog/CMS",
            description: "Posts, authors, categories, tags, comments, media",
            category: TemplateCategory::Comprehensive,
            tags: vec!["content", "articles", "publishing"],
        },
        SchemaTemplate {
            id: "saas_auth",
            name: "SaaS Authentication",
            description: "Organizations, users, roles, permissions, sessions, API keys",
            category: TemplateCategory::Comprehensive,
            tags: vec!["auth", "multi-tenant", "rbac"],
        },
        SchemaTemplate {
            id: "project_management",
            name: "Project Management",
            description: "Projects, tasks, milestones, team members, time tracking",
            category: TemplateCategory::Comprehensive,
            tags: vec!["tasks", "teams", "agile"],
        },
        SchemaTemplate {
            id: "social_media",
            name: "Social Media",
            description: "Users, posts, follows, likes, comments, notifications",
            category: TemplateCategory::Comprehensive,
            tags: vec!["social", "network", "feed"],
        },
        // Minimal Starters
        SchemaTemplate {
            id: "users_starter",
            name: "Users Table",
            description: "Basic users table with authentication fields",
            category: TemplateCategory::Starter,
            tags: vec!["users", "auth", "basic"],
        },
        SchemaTemplate {
            id: "basic_crud",
            name: "Basic CRUD",
            description: "Simple items table with standard CRUD fields",
            category: TemplateCategory::Starter,
            tags: vec!["crud", "simple", "items"],
        },
        SchemaTemplate {
            id: "audit_logging",
            name: "Audit Logging",
            description: "Audit log table for tracking changes",
            category: TemplateCategory::Starter,
            tags: vec!["audit", "logging", "tracking"],
        },
    ]
}

// Helper to create timestamp columns
fn timestamp_columns() -> Vec<ColumnModel> {
    vec![
        ColumnModel {
            name: "created_at".to_string(),
            data_type: "timestamptz".to_string(),
            is_nullable: false,
            default_value: Some("now()".to_string()),
            is_primary_key: false,
        },
        ColumnModel {
            name: "updated_at".to_string(),
            data_type: "timestamptz".to_string(),
            is_nullable: false,
            default_value: Some("now()".to_string()),
            is_primary_key: false,
        },
    ]
}

// =============================================================================
// E-commerce Template
// =============================================================================

fn generate_ecommerce_template() -> SchemaModel {
    let mut tables = HashMap::new();

    // Categories table
    tables.insert(
        "categories".to_string(),
        TableModel {
            name: "categories".to_string(),
            columns: vec![
                ColumnModel::primary_key("id", "bigserial"),
                ColumnModel {
                    name: "name".to_string(),
                    data_type: "varchar(100)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "slug".to_string(),
                    data_type: "varchar(100)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "parent_id".to_string(),
                    data_type: "bigint".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
            ]
            .into_iter()
            .chain(timestamp_columns())
            .collect(),
            constraints: vec![ConstraintModel::unique(
                "uq_categories_slug",
                vec!["slug".to_string()],
            )],
            indexes: vec![IndexModel::new(
                "idx_categories_parent",
                vec!["parent_id".to_string()],
                false,
            )],
            position: egui::pos2(50.0, 50.0),
            ..Default::default()
        },
    );

    // Products table
    tables.insert(
        "products".to_string(),
        TableModel {
            name: "products".to_string(),
            columns: vec![
                ColumnModel::primary_key("id", "bigserial"),
                ColumnModel {
                    name: "name".to_string(),
                    data_type: "varchar(255)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "description".to_string(),
                    data_type: "text".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "sku".to_string(),
                    data_type: "varchar(100)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "price".to_string(),
                    data_type: "decimal(10,2)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "stock_quantity".to_string(),
                    data_type: "integer".to_string(),
                    is_nullable: false,
                    default_value: Some("0".to_string()),
                    ..Default::default()
                },
                ColumnModel {
                    name: "category_id".to_string(),
                    data_type: "bigint".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "is_active".to_string(),
                    data_type: "boolean".to_string(),
                    is_nullable: false,
                    default_value: Some("true".to_string()),
                    ..Default::default()
                },
            ]
            .into_iter()
            .chain(timestamp_columns())
            .collect(),
            constraints: vec![
                ConstraintModel::unique("uq_products_sku", vec!["sku".to_string()]),
                ConstraintModel::foreign_key(
                    "fk_products_category",
                    "category_id",
                    "categories",
                    "id",
                    FkAction::SetNull,
                ),
            ],
            indexes: vec![
                IndexModel::new("idx_products_category", vec!["category_id".to_string()], false),
                IndexModel::new("idx_products_active", vec!["is_active".to_string()], false),
            ],
            position: egui::pos2(300.0, 50.0),
            ..Default::default()
        },
    );

    // Customers table
    tables.insert(
        "customers".to_string(),
        TableModel {
            name: "customers".to_string(),
            columns: vec![
                ColumnModel::primary_key("id", "bigserial"),
                ColumnModel {
                    name: "email".to_string(),
                    data_type: "varchar(255)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "password_hash".to_string(),
                    data_type: "varchar(255)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "first_name".to_string(),
                    data_type: "varchar(100)".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "last_name".to_string(),
                    data_type: "varchar(100)".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "phone".to_string(),
                    data_type: "varchar(20)".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
            ]
            .into_iter()
            .chain(timestamp_columns())
            .collect(),
            constraints: vec![ConstraintModel::unique(
                "uq_customers_email",
                vec!["email".to_string()],
            )],
            indexes: vec![],
            position: egui::pos2(50.0, 300.0),
            ..Default::default()
        },
    );

    // Orders table
    tables.insert(
        "orders".to_string(),
        TableModel {
            name: "orders".to_string(),
            columns: vec![
                ColumnModel::primary_key("id", "bigserial"),
                ColumnModel {
                    name: "customer_id".to_string(),
                    data_type: "bigint".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "status".to_string(),
                    data_type: "varchar(50)".to_string(),
                    is_nullable: false,
                    default_value: Some("'pending'".to_string()),
                    ..Default::default()
                },
                ColumnModel {
                    name: "total_amount".to_string(),
                    data_type: "decimal(12,2)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "shipping_address".to_string(),
                    data_type: "text".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "notes".to_string(),
                    data_type: "text".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
            ]
            .into_iter()
            .chain(timestamp_columns())
            .collect(),
            constraints: vec![ConstraintModel::foreign_key(
                "fk_orders_customer",
                "customer_id",
                "customers",
                "id",
                FkAction::Restrict,
            )],
            indexes: vec![
                IndexModel::new("idx_orders_customer", vec!["customer_id".to_string()], false),
                IndexModel::new("idx_orders_status", vec!["status".to_string()], false),
            ],
            position: egui::pos2(300.0, 300.0),
            ..Default::default()
        },
    );

    // Order items table
    tables.insert(
        "order_items".to_string(),
        TableModel {
            name: "order_items".to_string(),
            columns: vec![
                ColumnModel::primary_key("id", "bigserial"),
                ColumnModel {
                    name: "order_id".to_string(),
                    data_type: "bigint".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "product_id".to_string(),
                    data_type: "bigint".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "quantity".to_string(),
                    data_type: "integer".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "unit_price".to_string(),
                    data_type: "decimal(10,2)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
            ]
            .into_iter()
            .chain(timestamp_columns())
            .collect(),
            constraints: vec![
                ConstraintModel::foreign_key(
                    "fk_order_items_order",
                    "order_id",
                    "orders",
                    "id",
                    FkAction::Cascade,
                ),
                ConstraintModel::foreign_key(
                    "fk_order_items_product",
                    "product_id",
                    "products",
                    "id",
                    FkAction::Restrict,
                ),
            ],
            indexes: vec![
                IndexModel::new("idx_order_items_order", vec!["order_id".to_string()], false),
                IndexModel::new(
                    "idx_order_items_product",
                    vec!["product_id".to_string()],
                    false,
                ),
            ],
            position: egui::pos2(550.0, 300.0),
            ..Default::default()
        },
    );

    // Payments table
    tables.insert(
        "payments".to_string(),
        TableModel {
            name: "payments".to_string(),
            columns: vec![
                ColumnModel::primary_key("id", "bigserial"),
                ColumnModel {
                    name: "order_id".to_string(),
                    data_type: "bigint".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "amount".to_string(),
                    data_type: "decimal(12,2)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "payment_method".to_string(),
                    data_type: "varchar(50)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "status".to_string(),
                    data_type: "varchar(50)".to_string(),
                    is_nullable: false,
                    default_value: Some("'pending'".to_string()),
                    ..Default::default()
                },
                ColumnModel {
                    name: "transaction_id".to_string(),
                    data_type: "varchar(255)".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
            ]
            .into_iter()
            .chain(timestamp_columns())
            .collect(),
            constraints: vec![ConstraintModel::foreign_key(
                "fk_payments_order",
                "order_id",
                "orders",
                "id",
                FkAction::Cascade,
            )],
            indexes: vec![
                IndexModel::new("idx_payments_order", vec!["order_id".to_string()], false),
                IndexModel::new("idx_payments_status", vec!["status".to_string()], false),
            ],
            position: egui::pos2(550.0, 50.0),
            ..Default::default()
        },
    );

    SchemaModel { tables }
}

// =============================================================================
// Blog/CMS Template
// =============================================================================

fn generate_blog_cms_template() -> SchemaModel {
    let mut tables = HashMap::new();

    // Authors table
    tables.insert(
        "authors".to_string(),
        TableModel {
            name: "authors".to_string(),
            columns: vec![
                ColumnModel::primary_key("id", "bigserial"),
                ColumnModel {
                    name: "email".to_string(),
                    data_type: "varchar(255)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "password_hash".to_string(),
                    data_type: "varchar(255)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "name".to_string(),
                    data_type: "varchar(100)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "bio".to_string(),
                    data_type: "text".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "avatar_url".to_string(),
                    data_type: "varchar(500)".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
            ]
            .into_iter()
            .chain(timestamp_columns())
            .collect(),
            constraints: vec![ConstraintModel::unique(
                "uq_authors_email",
                vec!["email".to_string()],
            )],
            indexes: vec![],
            position: egui::pos2(50.0, 50.0),
            ..Default::default()
        },
    );

    // Categories table
    tables.insert(
        "categories".to_string(),
        TableModel {
            name: "categories".to_string(),
            columns: vec![
                ColumnModel::primary_key("id", "bigserial"),
                ColumnModel {
                    name: "name".to_string(),
                    data_type: "varchar(100)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "slug".to_string(),
                    data_type: "varchar(100)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "description".to_string(),
                    data_type: "text".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
            ]
            .into_iter()
            .chain(timestamp_columns())
            .collect(),
            constraints: vec![ConstraintModel::unique(
                "uq_categories_slug",
                vec!["slug".to_string()],
            )],
            indexes: vec![],
            position: egui::pos2(300.0, 50.0),
            ..Default::default()
        },
    );

    // Posts table
    tables.insert(
        "posts".to_string(),
        TableModel {
            name: "posts".to_string(),
            columns: vec![
                ColumnModel::primary_key("id", "bigserial"),
                ColumnModel {
                    name: "author_id".to_string(),
                    data_type: "bigint".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "category_id".to_string(),
                    data_type: "bigint".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "title".to_string(),
                    data_type: "varchar(255)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "slug".to_string(),
                    data_type: "varchar(255)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "content".to_string(),
                    data_type: "text".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "excerpt".to_string(),
                    data_type: "text".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "status".to_string(),
                    data_type: "varchar(20)".to_string(),
                    is_nullable: false,
                    default_value: Some("'draft'".to_string()),
                    ..Default::default()
                },
                ColumnModel {
                    name: "published_at".to_string(),
                    data_type: "timestamptz".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
            ]
            .into_iter()
            .chain(timestamp_columns())
            .collect(),
            constraints: vec![
                ConstraintModel::unique("uq_posts_slug", vec!["slug".to_string()]),
                ConstraintModel::foreign_key(
                    "fk_posts_author",
                    "author_id",
                    "authors",
                    "id",
                    FkAction::Restrict,
                ),
                ConstraintModel::foreign_key(
                    "fk_posts_category",
                    "category_id",
                    "categories",
                    "id",
                    FkAction::SetNull,
                ),
            ],
            indexes: vec![
                IndexModel::new("idx_posts_author", vec!["author_id".to_string()], false),
                IndexModel::new("idx_posts_category", vec!["category_id".to_string()], false),
                IndexModel::new("idx_posts_status", vec!["status".to_string()], false),
                IndexModel::new(
                    "idx_posts_published",
                    vec!["published_at".to_string()],
                    false,
                ),
            ],
            position: egui::pos2(50.0, 250.0),
            ..Default::default()
        },
    );

    // Tags table
    tables.insert(
        "tags".to_string(),
        TableModel {
            name: "tags".to_string(),
            columns: vec![
                ColumnModel::primary_key("id", "bigserial"),
                ColumnModel {
                    name: "name".to_string(),
                    data_type: "varchar(50)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "slug".to_string(),
                    data_type: "varchar(50)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
            ]
            .into_iter()
            .chain(timestamp_columns())
            .collect(),
            constraints: vec![ConstraintModel::unique(
                "uq_tags_slug",
                vec!["slug".to_string()],
            )],
            indexes: vec![],
            position: egui::pos2(550.0, 50.0),
            ..Default::default()
        },
    );

    // Post tags junction table
    tables.insert(
        "post_tags".to_string(),
        TableModel {
            name: "post_tags".to_string(),
            columns: vec![
                ColumnModel {
                    name: "post_id".to_string(),
                    data_type: "bigint".to_string(),
                    is_nullable: false,
                    is_primary_key: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "tag_id".to_string(),
                    data_type: "bigint".to_string(),
                    is_nullable: false,
                    is_primary_key: true,
                    ..Default::default()
                },
            ],
            constraints: vec![
                ConstraintModel::foreign_key(
                    "fk_post_tags_post",
                    "post_id",
                    "posts",
                    "id",
                    FkAction::Cascade,
                ),
                ConstraintModel::foreign_key(
                    "fk_post_tags_tag",
                    "tag_id",
                    "tags",
                    "id",
                    FkAction::Cascade,
                ),
            ],
            indexes: vec![IndexModel::new(
                "idx_post_tags_tag",
                vec!["tag_id".to_string()],
                false,
            )],
            position: egui::pos2(350.0, 250.0),
            ..Default::default()
        },
    );

    // Comments table
    tables.insert(
        "comments".to_string(),
        TableModel {
            name: "comments".to_string(),
            columns: vec![
                ColumnModel::primary_key("id", "bigserial"),
                ColumnModel {
                    name: "post_id".to_string(),
                    data_type: "bigint".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "parent_id".to_string(),
                    data_type: "bigint".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "author_name".to_string(),
                    data_type: "varchar(100)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "author_email".to_string(),
                    data_type: "varchar(255)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "content".to_string(),
                    data_type: "text".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "is_approved".to_string(),
                    data_type: "boolean".to_string(),
                    is_nullable: false,
                    default_value: Some("false".to_string()),
                    ..Default::default()
                },
            ]
            .into_iter()
            .chain(timestamp_columns())
            .collect(),
            constraints: vec![ConstraintModel::foreign_key(
                "fk_comments_post",
                "post_id",
                "posts",
                "id",
                FkAction::Cascade,
            )],
            indexes: vec![
                IndexModel::new("idx_comments_post", vec!["post_id".to_string()], false),
                IndexModel::new("idx_comments_parent", vec!["parent_id".to_string()], false),
            ],
            position: egui::pos2(50.0, 450.0),
            ..Default::default()
        },
    );

    SchemaModel { tables }
}

// =============================================================================
// SaaS Authentication Template
// =============================================================================

fn generate_saas_auth_template() -> SchemaModel {
    let mut tables = HashMap::new();

    // Organizations table
    tables.insert(
        "organizations".to_string(),
        TableModel {
            name: "organizations".to_string(),
            columns: vec![
                ColumnModel::primary_key("id", "bigserial"),
                ColumnModel {
                    name: "name".to_string(),
                    data_type: "varchar(255)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "slug".to_string(),
                    data_type: "varchar(100)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "plan".to_string(),
                    data_type: "varchar(50)".to_string(),
                    is_nullable: false,
                    default_value: Some("'free'".to_string()),
                    ..Default::default()
                },
                ColumnModel {
                    name: "is_active".to_string(),
                    data_type: "boolean".to_string(),
                    is_nullable: false,
                    default_value: Some("true".to_string()),
                    ..Default::default()
                },
            ]
            .into_iter()
            .chain(timestamp_columns())
            .collect(),
            constraints: vec![ConstraintModel::unique(
                "uq_organizations_slug",
                vec!["slug".to_string()],
            )],
            indexes: vec![],
            position: egui::pos2(50.0, 50.0),
            ..Default::default()
        },
    );

    // Roles table
    tables.insert(
        "roles".to_string(),
        TableModel {
            name: "roles".to_string(),
            columns: vec![
                ColumnModel::primary_key("id", "bigserial"),
                ColumnModel {
                    name: "organization_id".to_string(),
                    data_type: "bigint".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "name".to_string(),
                    data_type: "varchar(50)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "description".to_string(),
                    data_type: "text".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "is_system".to_string(),
                    data_type: "boolean".to_string(),
                    is_nullable: false,
                    default_value: Some("false".to_string()),
                    ..Default::default()
                },
            ]
            .into_iter()
            .chain(timestamp_columns())
            .collect(),
            constraints: vec![ConstraintModel::foreign_key(
                "fk_roles_org",
                "organization_id",
                "organizations",
                "id",
                FkAction::Cascade,
            )],
            indexes: vec![IndexModel::new(
                "idx_roles_org",
                vec!["organization_id".to_string()],
                false,
            )],
            position: egui::pos2(300.0, 50.0),
            ..Default::default()
        },
    );

    // Permissions table
    tables.insert(
        "permissions".to_string(),
        TableModel {
            name: "permissions".to_string(),
            columns: vec![
                ColumnModel::primary_key("id", "bigserial"),
                ColumnModel {
                    name: "name".to_string(),
                    data_type: "varchar(100)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "resource".to_string(),
                    data_type: "varchar(100)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "action".to_string(),
                    data_type: "varchar(50)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
            ]
            .into_iter()
            .chain(timestamp_columns())
            .collect(),
            constraints: vec![ConstraintModel::unique(
                "uq_permissions_resource_action",
                vec!["resource".to_string(), "action".to_string()],
            )],
            indexes: vec![],
            position: egui::pos2(550.0, 50.0),
            ..Default::default()
        },
    );

    // Role permissions junction
    tables.insert(
        "role_permissions".to_string(),
        TableModel {
            name: "role_permissions".to_string(),
            columns: vec![
                ColumnModel {
                    name: "role_id".to_string(),
                    data_type: "bigint".to_string(),
                    is_nullable: false,
                    is_primary_key: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "permission_id".to_string(),
                    data_type: "bigint".to_string(),
                    is_nullable: false,
                    is_primary_key: true,
                    ..Default::default()
                },
            ],
            constraints: vec![
                ConstraintModel::foreign_key(
                    "fk_role_perms_role",
                    "role_id",
                    "roles",
                    "id",
                    FkAction::Cascade,
                ),
                ConstraintModel::foreign_key(
                    "fk_role_perms_perm",
                    "permission_id",
                    "permissions",
                    "id",
                    FkAction::Cascade,
                ),
            ],
            indexes: vec![IndexModel::new(
                "idx_role_perms_perm",
                vec!["permission_id".to_string()],
                false,
            )],
            position: egui::pos2(450.0, 200.0),
            ..Default::default()
        },
    );

    // Users table
    tables.insert(
        "users".to_string(),
        TableModel {
            name: "users".to_string(),
            columns: vec![
                ColumnModel::primary_key("id", "bigserial"),
                ColumnModel {
                    name: "organization_id".to_string(),
                    data_type: "bigint".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "role_id".to_string(),
                    data_type: "bigint".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "email".to_string(),
                    data_type: "varchar(255)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "password_hash".to_string(),
                    data_type: "varchar(255)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "name".to_string(),
                    data_type: "varchar(100)".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "is_active".to_string(),
                    data_type: "boolean".to_string(),
                    is_nullable: false,
                    default_value: Some("true".to_string()),
                    ..Default::default()
                },
                ColumnModel {
                    name: "email_verified_at".to_string(),
                    data_type: "timestamptz".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
            ]
            .into_iter()
            .chain(timestamp_columns())
            .collect(),
            constraints: vec![
                ConstraintModel::unique("uq_users_email", vec!["email".to_string()]),
                ConstraintModel::foreign_key(
                    "fk_users_org",
                    "organization_id",
                    "organizations",
                    "id",
                    FkAction::Cascade,
                ),
                ConstraintModel::foreign_key(
                    "fk_users_role",
                    "role_id",
                    "roles",
                    "id",
                    FkAction::SetNull,
                ),
            ],
            indexes: vec![
                IndexModel::new("idx_users_org", vec!["organization_id".to_string()], false),
                IndexModel::new("idx_users_role", vec!["role_id".to_string()], false),
            ],
            position: egui::pos2(50.0, 300.0),
            ..Default::default()
        },
    );

    // Sessions table
    tables.insert(
        "sessions".to_string(),
        TableModel {
            name: "sessions".to_string(),
            columns: vec![
                ColumnModel::primary_key("id", "bigserial"),
                ColumnModel {
                    name: "user_id".to_string(),
                    data_type: "bigint".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "token_hash".to_string(),
                    data_type: "varchar(255)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "ip_address".to_string(),
                    data_type: "inet".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "user_agent".to_string(),
                    data_type: "text".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "expires_at".to_string(),
                    data_type: "timestamptz".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
            ]
            .into_iter()
            .chain(timestamp_columns())
            .collect(),
            constraints: vec![ConstraintModel::foreign_key(
                "fk_sessions_user",
                "user_id",
                "users",
                "id",
                FkAction::Cascade,
            )],
            indexes: vec![
                IndexModel::new("idx_sessions_user", vec!["user_id".to_string()], false),
                IndexModel::new("idx_sessions_token", vec!["token_hash".to_string()], true),
                IndexModel::new("idx_sessions_expires", vec!["expires_at".to_string()], false),
            ],
            position: egui::pos2(300.0, 300.0),
            ..Default::default()
        },
    );

    SchemaModel { tables }
}

// =============================================================================
// Project Management Template
// =============================================================================

fn generate_project_management_template() -> SchemaModel {
    let mut tables = HashMap::new();

    // Team members table
    tables.insert(
        "team_members".to_string(),
        TableModel {
            name: "team_members".to_string(),
            columns: vec![
                ColumnModel::primary_key("id", "bigserial"),
                ColumnModel {
                    name: "email".to_string(),
                    data_type: "varchar(255)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "name".to_string(),
                    data_type: "varchar(100)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "role".to_string(),
                    data_type: "varchar(50)".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "avatar_url".to_string(),
                    data_type: "varchar(500)".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
            ]
            .into_iter()
            .chain(timestamp_columns())
            .collect(),
            constraints: vec![ConstraintModel::unique(
                "uq_team_members_email",
                vec!["email".to_string()],
            )],
            indexes: vec![],
            position: egui::pos2(50.0, 50.0),
            ..Default::default()
        },
    );

    // Projects table
    tables.insert(
        "projects".to_string(),
        TableModel {
            name: "projects".to_string(),
            columns: vec![
                ColumnModel::primary_key("id", "bigserial"),
                ColumnModel {
                    name: "name".to_string(),
                    data_type: "varchar(255)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "description".to_string(),
                    data_type: "text".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "owner_id".to_string(),
                    data_type: "bigint".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "status".to_string(),
                    data_type: "varchar(30)".to_string(),
                    is_nullable: false,
                    default_value: Some("'active'".to_string()),
                    ..Default::default()
                },
                ColumnModel {
                    name: "start_date".to_string(),
                    data_type: "date".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "target_date".to_string(),
                    data_type: "date".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
            ]
            .into_iter()
            .chain(timestamp_columns())
            .collect(),
            constraints: vec![ConstraintModel::foreign_key(
                "fk_projects_owner",
                "owner_id",
                "team_members",
                "id",
                FkAction::Restrict,
            )],
            indexes: vec![
                IndexModel::new("idx_projects_owner", vec!["owner_id".to_string()], false),
                IndexModel::new("idx_projects_status", vec!["status".to_string()], false),
            ],
            position: egui::pos2(300.0, 50.0),
            ..Default::default()
        },
    );

    // Milestones table
    tables.insert(
        "milestones".to_string(),
        TableModel {
            name: "milestones".to_string(),
            columns: vec![
                ColumnModel::primary_key("id", "bigserial"),
                ColumnModel {
                    name: "project_id".to_string(),
                    data_type: "bigint".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "title".to_string(),
                    data_type: "varchar(255)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "description".to_string(),
                    data_type: "text".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "due_date".to_string(),
                    data_type: "date".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "is_completed".to_string(),
                    data_type: "boolean".to_string(),
                    is_nullable: false,
                    default_value: Some("false".to_string()),
                    ..Default::default()
                },
            ]
            .into_iter()
            .chain(timestamp_columns())
            .collect(),
            constraints: vec![ConstraintModel::foreign_key(
                "fk_milestones_project",
                "project_id",
                "projects",
                "id",
                FkAction::Cascade,
            )],
            indexes: vec![IndexModel::new(
                "idx_milestones_project",
                vec!["project_id".to_string()],
                false,
            )],
            position: egui::pos2(550.0, 50.0),
            ..Default::default()
        },
    );

    // Tasks table
    tables.insert(
        "tasks".to_string(),
        TableModel {
            name: "tasks".to_string(),
            columns: vec![
                ColumnModel::primary_key("id", "bigserial"),
                ColumnModel {
                    name: "project_id".to_string(),
                    data_type: "bigint".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "milestone_id".to_string(),
                    data_type: "bigint".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "assignee_id".to_string(),
                    data_type: "bigint".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "title".to_string(),
                    data_type: "varchar(255)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "description".to_string(),
                    data_type: "text".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "status".to_string(),
                    data_type: "varchar(30)".to_string(),
                    is_nullable: false,
                    default_value: Some("'todo'".to_string()),
                    ..Default::default()
                },
                ColumnModel {
                    name: "priority".to_string(),
                    data_type: "varchar(20)".to_string(),
                    is_nullable: false,
                    default_value: Some("'medium'".to_string()),
                    ..Default::default()
                },
                ColumnModel {
                    name: "due_date".to_string(),
                    data_type: "date".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "estimated_hours".to_string(),
                    data_type: "decimal(6,2)".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
            ]
            .into_iter()
            .chain(timestamp_columns())
            .collect(),
            constraints: vec![
                ConstraintModel::foreign_key(
                    "fk_tasks_project",
                    "project_id",
                    "projects",
                    "id",
                    FkAction::Cascade,
                ),
                ConstraintModel::foreign_key(
                    "fk_tasks_milestone",
                    "milestone_id",
                    "milestones",
                    "id",
                    FkAction::SetNull,
                ),
                ConstraintModel::foreign_key(
                    "fk_tasks_assignee",
                    "assignee_id",
                    "team_members",
                    "id",
                    FkAction::SetNull,
                ),
            ],
            indexes: vec![
                IndexModel::new("idx_tasks_project", vec!["project_id".to_string()], false),
                IndexModel::new("idx_tasks_milestone", vec!["milestone_id".to_string()], false),
                IndexModel::new("idx_tasks_assignee", vec!["assignee_id".to_string()], false),
                IndexModel::new("idx_tasks_status", vec!["status".to_string()], false),
            ],
            position: egui::pos2(300.0, 280.0),
            ..Default::default()
        },
    );

    // Time entries table
    tables.insert(
        "time_entries".to_string(),
        TableModel {
            name: "time_entries".to_string(),
            columns: vec![
                ColumnModel::primary_key("id", "bigserial"),
                ColumnModel {
                    name: "task_id".to_string(),
                    data_type: "bigint".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "user_id".to_string(),
                    data_type: "bigint".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "hours".to_string(),
                    data_type: "decimal(6,2)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "description".to_string(),
                    data_type: "text".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "logged_date".to_string(),
                    data_type: "date".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
            ]
            .into_iter()
            .chain(timestamp_columns())
            .collect(),
            constraints: vec![
                ConstraintModel::foreign_key(
                    "fk_time_entries_task",
                    "task_id",
                    "tasks",
                    "id",
                    FkAction::Cascade,
                ),
                ConstraintModel::foreign_key(
                    "fk_time_entries_user",
                    "user_id",
                    "team_members",
                    "id",
                    FkAction::Restrict,
                ),
            ],
            indexes: vec![
                IndexModel::new("idx_time_entries_task", vec!["task_id".to_string()], false),
                IndexModel::new("idx_time_entries_user", vec!["user_id".to_string()], false),
                IndexModel::new(
                    "idx_time_entries_date",
                    vec!["logged_date".to_string()],
                    false,
                ),
            ],
            position: egui::pos2(50.0, 280.0),
            ..Default::default()
        },
    );

    SchemaModel { tables }
}

// =============================================================================
// Social Media Template
// =============================================================================

fn generate_social_media_template() -> SchemaModel {
    let mut tables = HashMap::new();

    // Users table
    tables.insert(
        "users".to_string(),
        TableModel {
            name: "users".to_string(),
            columns: vec![
                ColumnModel::primary_key("id", "bigserial"),
                ColumnModel {
                    name: "username".to_string(),
                    data_type: "varchar(50)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "email".to_string(),
                    data_type: "varchar(255)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "password_hash".to_string(),
                    data_type: "varchar(255)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "display_name".to_string(),
                    data_type: "varchar(100)".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "bio".to_string(),
                    data_type: "text".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "avatar_url".to_string(),
                    data_type: "varchar(500)".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "is_verified".to_string(),
                    data_type: "boolean".to_string(),
                    is_nullable: false,
                    default_value: Some("false".to_string()),
                    ..Default::default()
                },
            ]
            .into_iter()
            .chain(timestamp_columns())
            .collect(),
            constraints: vec![
                ConstraintModel::unique("uq_users_username", vec!["username".to_string()]),
                ConstraintModel::unique("uq_users_email", vec!["email".to_string()]),
            ],
            indexes: vec![],
            position: egui::pos2(50.0, 50.0),
            ..Default::default()
        },
    );

    // Posts table
    tables.insert(
        "posts".to_string(),
        TableModel {
            name: "posts".to_string(),
            columns: vec![
                ColumnModel::primary_key("id", "bigserial"),
                ColumnModel {
                    name: "user_id".to_string(),
                    data_type: "bigint".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "content".to_string(),
                    data_type: "text".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "media_urls".to_string(),
                    data_type: "text[]".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "like_count".to_string(),
                    data_type: "integer".to_string(),
                    is_nullable: false,
                    default_value: Some("0".to_string()),
                    ..Default::default()
                },
                ColumnModel {
                    name: "comment_count".to_string(),
                    data_type: "integer".to_string(),
                    is_nullable: false,
                    default_value: Some("0".to_string()),
                    ..Default::default()
                },
                ColumnModel {
                    name: "is_public".to_string(),
                    data_type: "boolean".to_string(),
                    is_nullable: false,
                    default_value: Some("true".to_string()),
                    ..Default::default()
                },
            ]
            .into_iter()
            .chain(timestamp_columns())
            .collect(),
            constraints: vec![ConstraintModel::foreign_key(
                "fk_posts_user",
                "user_id",
                "users",
                "id",
                FkAction::Cascade,
            )],
            indexes: vec![
                IndexModel::new("idx_posts_user", vec!["user_id".to_string()], false),
                IndexModel::new("idx_posts_created", vec!["created_at".to_string()], false),
            ],
            position: egui::pos2(300.0, 50.0),
            ..Default::default()
        },
    );

    // Follows table
    tables.insert(
        "follows".to_string(),
        TableModel {
            name: "follows".to_string(),
            columns: vec![
                ColumnModel {
                    name: "follower_id".to_string(),
                    data_type: "bigint".to_string(),
                    is_nullable: false,
                    is_primary_key: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "following_id".to_string(),
                    data_type: "bigint".to_string(),
                    is_nullable: false,
                    is_primary_key: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "created_at".to_string(),
                    data_type: "timestamptz".to_string(),
                    is_nullable: false,
                    default_value: Some("now()".to_string()),
                    ..Default::default()
                },
            ],
            constraints: vec![
                ConstraintModel::foreign_key(
                    "fk_follows_follower",
                    "follower_id",
                    "users",
                    "id",
                    FkAction::Cascade,
                ),
                ConstraintModel::foreign_key(
                    "fk_follows_following",
                    "following_id",
                    "users",
                    "id",
                    FkAction::Cascade,
                ),
            ],
            indexes: vec![IndexModel::new(
                "idx_follows_following",
                vec!["following_id".to_string()],
                false,
            )],
            position: egui::pos2(50.0, 280.0),
            ..Default::default()
        },
    );

    // Likes table
    tables.insert(
        "likes".to_string(),
        TableModel {
            name: "likes".to_string(),
            columns: vec![
                ColumnModel {
                    name: "user_id".to_string(),
                    data_type: "bigint".to_string(),
                    is_nullable: false,
                    is_primary_key: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "post_id".to_string(),
                    data_type: "bigint".to_string(),
                    is_nullable: false,
                    is_primary_key: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "created_at".to_string(),
                    data_type: "timestamptz".to_string(),
                    is_nullable: false,
                    default_value: Some("now()".to_string()),
                    ..Default::default()
                },
            ],
            constraints: vec![
                ConstraintModel::foreign_key(
                    "fk_likes_user",
                    "user_id",
                    "users",
                    "id",
                    FkAction::Cascade,
                ),
                ConstraintModel::foreign_key(
                    "fk_likes_post",
                    "post_id",
                    "posts",
                    "id",
                    FkAction::Cascade,
                ),
            ],
            indexes: vec![IndexModel::new(
                "idx_likes_post",
                vec!["post_id".to_string()],
                false,
            )],
            position: egui::pos2(550.0, 50.0),
            ..Default::default()
        },
    );

    // Notifications table
    tables.insert(
        "notifications".to_string(),
        TableModel {
            name: "notifications".to_string(),
            columns: vec![
                ColumnModel::primary_key("id", "bigserial"),
                ColumnModel {
                    name: "user_id".to_string(),
                    data_type: "bigint".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "actor_id".to_string(),
                    data_type: "bigint".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "type".to_string(),
                    data_type: "varchar(50)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "reference_id".to_string(),
                    data_type: "bigint".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "message".to_string(),
                    data_type: "text".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "is_read".to_string(),
                    data_type: "boolean".to_string(),
                    is_nullable: false,
                    default_value: Some("false".to_string()),
                    ..Default::default()
                },
            ]
            .into_iter()
            .chain(timestamp_columns())
            .collect(),
            constraints: vec![
                ConstraintModel::foreign_key(
                    "fk_notifications_user",
                    "user_id",
                    "users",
                    "id",
                    FkAction::Cascade,
                ),
                ConstraintModel::foreign_key(
                    "fk_notifications_actor",
                    "actor_id",
                    "users",
                    "id",
                    FkAction::SetNull,
                ),
            ],
            indexes: vec![
                IndexModel::new(
                    "idx_notifications_user",
                    vec!["user_id".to_string()],
                    false,
                ),
                IndexModel::new(
                    "idx_notifications_unread",
                    vec!["user_id".to_string(), "is_read".to_string()],
                    false,
                ),
            ],
            position: egui::pos2(300.0, 280.0),
            ..Default::default()
        },
    );

    SchemaModel { tables }
}

// =============================================================================
// Minimal Starters
// =============================================================================

fn generate_users_starter_template() -> SchemaModel {
    let mut tables = HashMap::new();

    tables.insert(
        "users".to_string(),
        TableModel {
            name: "users".to_string(),
            columns: vec![
                ColumnModel::primary_key("id", "bigserial"),
                ColumnModel {
                    name: "email".to_string(),
                    data_type: "varchar(255)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "password_hash".to_string(),
                    data_type: "varchar(255)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "name".to_string(),
                    data_type: "varchar(100)".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "is_active".to_string(),
                    data_type: "boolean".to_string(),
                    is_nullable: false,
                    default_value: Some("true".to_string()),
                    ..Default::default()
                },
                ColumnModel {
                    name: "last_login_at".to_string(),
                    data_type: "timestamptz".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
            ]
            .into_iter()
            .chain(timestamp_columns())
            .collect(),
            constraints: vec![ConstraintModel::unique(
                "uq_users_email",
                vec!["email".to_string()],
            )],
            indexes: vec![IndexModel::new(
                "idx_users_active",
                vec!["is_active".to_string()],
                false,
            )],
            position: egui::pos2(50.0, 50.0),
            ..Default::default()
        },
    );

    SchemaModel { tables }
}

fn generate_basic_crud_template() -> SchemaModel {
    let mut tables = HashMap::new();

    tables.insert(
        "items".to_string(),
        TableModel {
            name: "items".to_string(),
            columns: vec![
                ColumnModel::primary_key("id", "bigserial"),
                ColumnModel {
                    name: "name".to_string(),
                    data_type: "varchar(255)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "description".to_string(),
                    data_type: "text".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "status".to_string(),
                    data_type: "varchar(50)".to_string(),
                    is_nullable: false,
                    default_value: Some("'active'".to_string()),
                    ..Default::default()
                },
                ColumnModel {
                    name: "metadata".to_string(),
                    data_type: "jsonb".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
            ]
            .into_iter()
            .chain(timestamp_columns())
            .collect(),
            constraints: vec![],
            indexes: vec![IndexModel::new(
                "idx_items_status",
                vec!["status".to_string()],
                false,
            )],
            position: egui::pos2(50.0, 50.0),
            ..Default::default()
        },
    );

    SchemaModel { tables }
}

fn generate_audit_logging_template() -> SchemaModel {
    let mut tables = HashMap::new();

    tables.insert(
        "audit_logs".to_string(),
        TableModel {
            name: "audit_logs".to_string(),
            columns: vec![
                ColumnModel::primary_key("id", "bigserial"),
                ColumnModel {
                    name: "entity_type".to_string(),
                    data_type: "varchar(100)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "entity_id".to_string(),
                    data_type: "varchar(100)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "action".to_string(),
                    data_type: "varchar(50)".to_string(),
                    is_nullable: false,
                    ..Default::default()
                },
                ColumnModel {
                    name: "actor_id".to_string(),
                    data_type: "varchar(100)".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "actor_type".to_string(),
                    data_type: "varchar(50)".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "old_values".to_string(),
                    data_type: "jsonb".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "new_values".to_string(),
                    data_type: "jsonb".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "ip_address".to_string(),
                    data_type: "inet".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "user_agent".to_string(),
                    data_type: "text".to_string(),
                    is_nullable: true,
                    ..Default::default()
                },
                ColumnModel {
                    name: "created_at".to_string(),
                    data_type: "timestamptz".to_string(),
                    is_nullable: false,
                    default_value: Some("now()".to_string()),
                    ..Default::default()
                },
            ],
            constraints: vec![],
            indexes: vec![
                IndexModel::new(
                    "idx_audit_logs_entity",
                    vec!["entity_type".to_string(), "entity_id".to_string()],
                    false,
                ),
                IndexModel::new(
                    "idx_audit_logs_actor",
                    vec!["actor_type".to_string(), "actor_id".to_string()],
                    false,
                ),
                IndexModel::new(
                    "idx_audit_logs_created",
                    vec!["created_at".to_string()],
                    false,
                ),
            ],
            position: egui::pos2(50.0, 50.0),
            ..Default::default()
        },
    );

    SchemaModel { tables }
}
