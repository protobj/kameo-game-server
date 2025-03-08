
/*!
<auto-generated>
    This code was generated by a tool.
    Changes to this file may cause incorrect behavior and will be lost if
    the code is regenerated.
</auto-generated>
*/


use super::*;
use luban_lib::*;

#[derive(Debug)]
#[derive(macros::TryIntoBase)]
pub struct RoleInfo {
    pub x1: i32,
    pub x3: i32,
    pub role_id: i64,
}

impl RoleInfo{
    pub fn new(mut buf: &mut ByteBuf) -> Result<RoleInfo, LubanError> {
        let x1 = buf.read_int();
        let x3 = buf.read_int();
        let role_id = buf.read_long();
        
        Ok(RoleInfo { x1, x3, role_id, })
    }

    pub const __ID__: i32 = -989153243;
}


