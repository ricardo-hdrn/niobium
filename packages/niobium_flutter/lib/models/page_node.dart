/// A node in a page layout tree.
///
/// Pages mix content (markdown, text, dividers) with input fields
/// and layout containers (sections). The tree is rendered recursively.
class PageNode {
  final String type;
  final String? content;
  final String? title;
  final String? key;
  final Map<String, dynamic>? field;
  final List<PageNode>? children;

  PageNode({
    required this.type,
    this.content,
    this.title,
    this.key,
    this.field,
    this.children,
  });

  factory PageNode.fromJson(Map<String, dynamic> json) {
    return PageNode(
      type: json['type'] as String,
      content: json['content'] as String?,
      title: json['title'] as String?,
      key: json['key'] as String?,
      field: json['field'] as Map<String, dynamic>?,
      children: (json['children'] as List<dynamic>?)
          ?.map((c) => PageNode.fromJson(c as Map<String, dynamic>))
          .toList(),
    );
  }

  bool get isInput => type == 'input' && key != null && field != null;
}

/// Check if any node in the tree is an input.
bool hasInputNodes(List<PageNode> nodes) {
  for (final node in nodes) {
    if (node.isInput) return true;
    if (node.children != null && hasInputNodes(node.children!)) return true;
  }
  return false;
}
