use std::fmt;

/// Unadorned reg is a 64-bit reg
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Reg {
    Rax,
    Rbx,
    Rdx,
    Rcx,
    Rsi,
    Rdi,
    Rsp,
    Rbp,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Reg8 {
    Ah,
    Al,
    Ch,
    Cl,
    Dh,
    Dl,
    Bh,
    Bl,
    Spl,
    Bpl,
    Sil,
    Dil,
    R8b,
    R9b,
    R10b,
    R11b,
    R12b,
    R13b,
    R14b,
    R15b,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConditionCode {
    E,
    NE,
    L,
    LE,
    G,
    GE,
    S,
    Z,
    NZ,
    O,
    NO,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemRef {
    pub reg: Reg,
    pub offset: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arg64 {
    Reg(Reg),
    Signed(i64),
    Unsigned(u64),
    Mem(MemRef),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arg32 {
    Reg(Reg),
    Signed(i32),
    Unsigned(u32),
    Mem(MemRef),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reg32 {
    Reg(Reg),
    Imm(i32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MovArgs {
    ToReg(Reg, Arg64),
    ToMem(MemRef, Reg32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinArgs {
    ToReg(Reg, Arg32),
    ToMem(MemRef, Reg32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Loc {
    Reg(Reg),
    Mem(MemRef),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Instr {
    Mov(MovArgs),

    Add(BinArgs),
    Sub(BinArgs),
    IMul(BinArgs),
    And(BinArgs),
    Or(BinArgs),
    Xor(BinArgs),
    Shr(BinArgs),
    Sar(BinArgs),
    Cmp(BinArgs),
    Test(BinArgs),

    Push(Arg32),
    Pop(Loc),

    Label(String),
    Comment(String),
    Section(String),
    Global(String),
    Extern(String),
    // TODO: these should not be required to be strings
    Call(String),
    Ret,
    Jmp(String),

    // Conditional mov, jmp and set
    CMovCC(ConditionCode, BinArgs),
    JCC(ConditionCode, String),
    SetCC(ConditionCode, Reg8),
}

impl fmt::Display for ConditionCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ConditionCode::*;
        match self {
            E => write!(f, "e"),
            NE => write!(f, "ne"),
            L => write!(f, "l"),
            LE => write!(f, "le"),
            G => write!(f, "g"),
            GE => write!(f, "ge"),
            S => write!(f, "s"),
            Z => write!(f, "z"),
            NZ => write!(f, "nz"),
            O => write!(f, "o"),
            NO => write!(f, "no"),
        }
    }
}

impl fmt::Display for Reg8 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Reg8::*;
        match self {
            Al => write!(f, "al"),
            Ah => write!(f, "ah"),
            Cl => write!(f, "cl"),
            Ch => write!(f, "ch"),
            Dl => write!(f, "dl"),
            Dh => write!(f, "dh"),
            Bl => write!(f, "bl"),
            Bh => write!(f, "bh"),
            Spl => write!(f, "spl"),
            Bpl => write!(f, "bpl"),
            Sil => write!(f, "sil"),
            Dil => write!(f, "dil"),
            R8b => write!(f, "r8b"),
            R9b => write!(f, "r9b"),
            R10b => write!(f, "r10b"),
            R11b => write!(f, "r11b"),
            R12b => write!(f, "r12b"),
            R13b => write!(f, "r13b"),
            R14b => write!(f, "r14b"),
            R15b => write!(f, "r15b"),
        }
    }
}

pub fn reg_to_string(r: Reg) -> String {
    match r {
        Reg::Rax => String::from("rax"),
        Reg::Rbx => String::from("rbx"),
        Reg::Rcx => String::from("rcx"),
        Reg::Rdx => String::from("rdx"),
        Reg::Rsi => String::from("rsi"),
        Reg::Rdi => String::from("rdi"),
        Reg::Rsp => String::from("rsp"),
        Reg::Rbp => String::from("rbp"),
        Reg::R8 => String::from("r8"),
        Reg::R9 => String::from("r9"),
        Reg::R10 => String::from("r10"),
        Reg::R11 => String::from("r11"),
        Reg::R12 => String::from("r12"),
        Reg::R13 => String::from("r13"),
        Reg::R14 => String::from("r14"),
        Reg::R15 => String::from("r15"),
    }
}

pub fn imm32_to_string(i: i32) -> String {
    i.to_string()
}

pub fn mem_ref_to_string(m: MemRef) -> String {
    format!("QWORD [{} + {}]", reg_to_string(m.reg), m.offset)
}

pub fn reg32_to_string(r_or_i: Reg32) -> String {
    match r_or_i {
        Reg32::Reg(r) => reg_to_string(r),
        Reg32::Imm(i) => imm32_to_string(i),
    }
}

pub fn arg32_to_string(arg: Arg32) -> String {
    match arg {
        Arg32::Reg(r) => reg_to_string(r),
        Arg32::Signed(i) => imm32_to_string(i),
        Arg32::Unsigned(u) => format!("0x{:08x}", u),
        Arg32::Mem(m) => mem_ref_to_string(m),
    }
}

pub fn arg64_to_string(arg: Arg64) -> String {
    match arg {
        Arg64::Reg(r) => reg_to_string(r),
        Arg64::Signed(i) => i.to_string(),
        Arg64::Unsigned(u) => format!("0x{:016x}", u),
        Arg64::Mem(m) => mem_ref_to_string(m),
    }
}

pub fn mov_args_to_string(args: MovArgs) -> String {
    match args {
        MovArgs::ToReg(r, arg) => {
            format!("{}, {}", reg_to_string(r), arg64_to_string(arg))
        }
        MovArgs::ToMem(mem, arg) => {
            format!("{}, {}", mem_ref_to_string(mem), reg32_to_string(arg))
        }
    }
}

pub fn bin_args_to_string(args: BinArgs) -> String {
    match args {
        BinArgs::ToReg(r, arg) => {
            format!("{}, {}", reg_to_string(r), arg32_to_string(arg))
        }
        BinArgs::ToMem(mem, arg) => {
            format!("{}, {}", mem_ref_to_string(mem), reg32_to_string(arg))
        }
    }
}

pub fn loc_to_string(loc: Loc) -> String {
    match loc {
        Loc::Reg(r) => reg_to_string(r),
        Loc::Mem(m) => mem_ref_to_string(m),
    }
}

pub fn instr_to_string(i: &Instr) -> String {
    match i {
        Instr::Mov(args) => {
            format!("        mov {}", mov_args_to_string(*args))
        }
        Instr::Add(args) => {
            format!("        add {}", bin_args_to_string(*args))
        }
        Instr::Sub(args) => {
            format!("        sub {}", bin_args_to_string(*args))
        }
        Instr::IMul(args) => {
            format!("        imul {}", bin_args_to_string(*args))
        }
        Instr::And(args) => {
            format!("        and {}", bin_args_to_string(*args))
        }
        Instr::Or(args) => {
            format!("        or {}", bin_args_to_string(*args))
        }
        Instr::Xor(args) => {
            format!("        xor {}", bin_args_to_string(*args))
        }
        Instr::Shr(args) => {
            format!("        shr {}", bin_args_to_string(*args))
        }
        Instr::Sar(args) => {
            format!("        sar {}", bin_args_to_string(*args))
        }
        Instr::Cmp(args) => {
            format!("        cmp {}", bin_args_to_string(*args))
        }
        Instr::Test(args) => {
            format!("        test {}", bin_args_to_string(*args))
        }
        Instr::Push(arg) => {
            format!("        push {}", arg32_to_string(*arg))
        }
        Instr::Pop(loc) => {
            format!("        pop {}", loc_to_string(*loc))
        }
        Instr::Label(s) => {
            format!("{}:", s)
        }
        Instr::Comment(s) => {
            format!(";;; {}", s)
        }
        Instr::Section(s) => {
            format!("section {}", s)
        }
        Instr::Global(s) => {
            format!("        global {}", s)
        }
        Instr::Extern(s) => {
            format!("        extern {}", s)
        }

        Instr::Call(s) => {
            format!("        call {}", s)
        }
        Instr::Ret => {
            format!("        ret")
        }

        Instr::CMovCC(cc, args) => {
            format!("        cmov{} {}", cc, bin_args_to_string(*args))
        }

        Instr::Jmp(s) => {
            format!("        jmp {}", s)
        }
        Instr::JCC(cc, l) => {
            format!("        j{} {}", cc, l)
        }
        Instr::SetCC(cc, a) => {
            format!("        set{} {}", cc, a)
        }
    }
}

pub fn instrs_to_string(is: &[Instr]) -> String {
    let mut buf = String::new();
    for i in is {
        buf.push_str(&instr_to_string(&i));
        buf.push_str("\n");
    }
    buf
}
