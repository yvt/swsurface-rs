//! OpenGL functions imported from the `OpenGL` system framework.
use std::os::raw::{c_float, c_int, c_uint, c_void};

pub type GLfloat = c_float;
pub type GLint = c_int;
pub type GLuint = c_uint;
pub type GLbitfield = c_uint;
pub type GLclampf = c_float;
pub type GLenum = c_int;
pub type GLsizei = c_int;
pub type GLvoid = c_void;

pub const GL_COLOR_BUFFER_BIT: GLbitfield = 0x00004000;
pub const GL_TRIANGLE_STRIP: GLenum = 0x0005;
pub const GL_TEXTURE_2D: GLenum = 0x0DE1;
pub const GL_TEXTURE_MAG_FILTER: GLenum = 0x2800;
pub const GL_TEXTURE_MIN_FILTER: GLenum = 0x2801;
pub const GL_LINEAR: GLenum = 0x2601;
pub const GL_BGRA: GLenum = 0x80E1;
pub const GL_RGBA: GLenum = 0x1908;
pub const GL_RGB: GLenum = 0x1907;
pub const GL_UNSIGNED_BYTE: GLenum = 0x1401;
pub const GL_UNSIGNED_INT_8_8_8_8_REV: GLenum = 0x8367;
pub const GL_UNPACK_ROW_LENGTH: GLenum = 0x0CF2;

#[link(name = "OpenGL", kind = "framework")]
extern "C" {
    pub fn glClear(mask: GLbitfield);
    pub fn glClearColor(red: GLclampf, green: GLclampf, blue: GLclampf, alpha: GLclampf);

    pub fn glBegin(mode: GLenum);
    pub fn glEnd();
    pub fn glVertex2f(x: GLfloat, y: GLfloat);
    pub fn glTexCoord2f(x: GLfloat, y: GLfloat);

    pub fn glEnable(cap: GLenum);

    pub fn glTexImage2D(
        target: GLenum,
        level: GLint,
        internalFormat: GLint,
        width: GLsizei,
        height: GLsizei,
        border: GLint,
        format: GLenum,
        ty: GLenum,
        pixels: *const GLvoid,
    );
    pub fn glTexSubImage2D(
        target: GLenum,
        level: GLint,
        xoffset: GLint,
        yoffset: GLint,
        width: GLsizei,
        height: GLsizei,
        format: GLenum,
        ty: GLenum,
        pixels: *const GLvoid,
    );
    pub fn glGenTextures(n: GLsizei, textures: *mut GLuint);
    pub fn glBindTexture(target: GLenum, texture: GLuint);
    pub fn glTexParameteri(target: GLenum, pname: GLenum, param: GLint);
    pub fn glPixelStorei(pname: GLenum, param: GLint);
}
