
impl crate::store_cmd::StoreInfoReq {
    pub const CMD: i32 = 1101;

    pub const fn cmd(&self) -> i32 {
        1101
    }
}
                    
impl crate::store_cmd::StoreInfoRsp {
    pub const CMD: i32 = 1102;

    pub const fn cmd(&self) -> i32 {
        1102
    }
}
                    
impl crate::login_cmd::LoginReq {
    pub const CMD: i32 = 1001;

    pub const fn cmd(&self) -> i32 {
        1001
    }
}
                    
impl crate::login_cmd::LoginRsp {
    pub const CMD: i32 = 1002;

    pub const fn cmd(&self) -> i32 {
        1002
    }
}
                    
impl crate::login_cmd::RegisterReq {
    pub const CMD: i32 = 1003;

    pub const fn cmd(&self) -> i32 {
        1003
    }
}
                    
impl crate::login_cmd::RegisterRsp {
    pub const CMD: i32 = 1004;

    pub const fn cmd(&self) -> i32 {
        1004
    }
}
                    
impl crate::login_cmd::LogoutReq {
    pub const CMD: i32 = 1005;

    pub const fn cmd(&self) -> i32 {
        1005
    }
}
                    
impl crate::login_cmd::LogoutRsp {
    pub const CMD: i32 = 1006;

    pub const fn cmd(&self) -> i32 {
        1006
    }
}
                    
impl crate::base_cmd::ErrorRsp {
    pub const CMD: i32 = 601;

    pub const fn cmd(&self) -> i32 {
        601
    }
}
                    