use diesel::{
    prelude::{Insertable, Queryable},
    Selectable,
};
use serenity::all::{GuildId, RoleId};
use uuid::Uuid;

#[derive(Insertable)]
#[diesel(table_name = crate::schema::allowed_roles)]
pub struct CreateAllowedRole {
    guild: i64,
    role_id: i64,
    operator_role: i64,
}

impl CreateAllowedRole {
    pub fn new<G: Into<GuildId>, R1: Into<RoleId>, R2: Into<RoleId>>(
        guild: G,
        operator_role: R1,
        role: R2,
    ) -> Self {
        CreateAllowedRole {
            guild: guild.into().get().try_into().unwrap(),
            role_id: role.into().get().try_into().unwrap(),
            operator_role: operator_role.into().get().try_into().unwrap(),
        }
    }
}

#[allow(dead_code)]
#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::allowed_roles)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AllowedRole {
    pub id: Uuid,
    pub guild: i64,
    pub role_id: i64,
    pub operator_role: i64,
}
