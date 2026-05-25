use std::fs;

use super::{CompileOptions, Compiler, OutputKind, unique_output_path};

#[test]
fn writes_llvm_ir_for_type_declaration_and_instantiation() {
    let source = r#"
type Node(value: Number, left: Node, right: Node) {
    value = value;
    left = left;
    right = right;

    setValue(v: Number) => { self.value := v};
    setLeft(l: Node) => {self.left := l};
    setRight(r: Node) =>{ self.right := r};

    insert(x: Number) => {
	if (x <= self.value)
        if (self.left == null)
            self.setLeft(new Node(x, null, null))
        else
            self.left.insert(x)
    else 
        if (self.right == null)
            self.setRight(new Node(x, null, null))
        else
            self.right.insert(x)

	};


   inorder() => {
    if (self.left == null && self.right == null)
        self.value @ ""
    elif (self.left == null)
        self.value @ " " @ self.right.inorder()
    elif (self.right == null)
        self.left.inorder() @ " " @ self.value
    else
        self.left.inorder() @ " " @ self.value @ " " @ self.right.inorder();
	};

	show(prefix: String, isLeft: Boolean) => {

        let result = "";

        if (self.right != null) result := result @ 
			self.right.show(prefix @ (if (isLeft) "│   " else "    "), false) 
		else  "4";

        result := result @ prefix @ (if (isLeft) "└── " else "┌── ") @ self.value @ "\n";

        if (self.left != null) result := result @ 
			self.left.show(prefix @ (if (isLeft) "    " else "│   "), true)
		else "4";
        result

    };

    tree() => {

        self.show("", true)

    };

}

let root = new Node(8, null, null);
root.insert(3);
root.insert(10);
root.insert(1);
root.insert(6);
root.insert(14);
root.insert(4);
root.insert(7);
root.insert(13);
root.insert(0);
print("in:   " @ root.inorder());
print("\n");
print(root.tree());

"#;
    let output_path = unique_output_path("type_feature_ir");

    let mut compiler = Compiler::new();
    let report = compiler.compile(
        source,
        &CompileOptions {
            output_path: output_path.clone(),
        },
    );

    assert!(
        report.errors.is_empty(),
        "expected successful compilation, got errors: {:?}",
        report.errors
    );
    assert_eq!(report.output_kind, Some(OutputKind::LlvmIr));

    let llvm_ir = fs::read_to_string(&output_path)
        .expect("compiler should write llvm output file on success");
    assert!(
        llvm_ir.contains("call i8* @malloc(i64"),
        "output should contain object allocation, got:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("norm(i8* %self)"),
        "output should contain method definition with receiver, got:\n{}",
        llvm_ir
    );
}
