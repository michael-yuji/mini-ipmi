#[macro_export]
macro_rules! take {
    ($slice:expr,$idx:expr,$cnt:literal) => {
        {
            let r = &$slice[$idx..($idx + $cnt)];
            $idx += $cnt;
            r
        }
    }
}

#[macro_export]
macro_rules! take_u8 {
    ($slice:expr,$idx:expr) => {
        {
            let r = $slice[$idx];
            $idx += 1;
            r
        }
    }
}

#[macro_export]
macro_rules! take_be_u32 {
    ($slice:expr,$idx:expr) => {
        {
            let var = crate::take!($slice, $idx, 4);
            u32::from_be_bytes(var.try_into().unwrap())
        }
    }
}

#[macro_export]
macro_rules! take_le_u32 {
    ($slice:expr,$idx:expr) => {
        {
            let var = crate::take!($slice, $idx, 4);
            u32::from_le_bytes(var.try_into().unwrap())
        }
    }

}

#[macro_export]
macro_rules! take_remain {
    ($slice:expr,$idx:expr) => {
        &$slice[$idx..]
    }
}

