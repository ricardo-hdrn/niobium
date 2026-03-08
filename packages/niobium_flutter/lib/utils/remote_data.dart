// Shared utilities for fetching remote options from x-source endpoints.
//
// Used by all scale-aware select widgets (dropdown, searchable, autocomplete, modal).

import 'dart:convert';
import 'package:http/http.dart' as http;
import '../models/form_schema.dart';

/// A single option fetched from a remote endpoint.
class RemoteOption {
  final String label;
  final dynamic value;
  RemoteOption({required this.label, required this.value});
}

/// Result of a remote fetch, including total count if available.
class FetchResult {
  final List<RemoteOption> options;
  final int? totalCount; // from response headers or wrapper
  FetchResult({required this.options, this.totalCount});
}

/// Fetch options from a [FieldDataSource] endpoint.
///
/// Supports optional query parameters for search, pagination, and filters.
Future<FetchResult> fetchRemoteOptions(
  FieldDataSource source, {
  String? searchQuery,
  int? page,
  Map<String, String>? filterValues,
}) async {
  var uri = Uri.parse(source.url);

  // Build query parameters
  final params = Map<String, String>.from(uri.queryParameters);
  if (searchQuery != null && searchQuery.isNotEmpty && source.searchParam != null) {
    params[source.searchParam!] = searchQuery;
  }
  if (page != null && source.pageParam != null) {
    params[source.pageParam!] = page.toString();
  }
  if (source.pageSize != null && source.pageParam != null) {
    params['_limit'] = source.pageSize.toString();
  }
  if (filterValues != null) {
    params.addAll(filterValues);
  }

  uri = uri.replace(queryParameters: params.isNotEmpty ? params : null);

  final response = await http.get(uri, headers: source.headers);

  if (response.statusCode != 200) {
    throw RemoteDataException('HTTP ${response.statusCode}');
  }

  final dynamic body = jsonDecode(response.body);

  // Navigate to the array using dot-notation path
  dynamic items = body;
  if (source.path != null && source.path!.isNotEmpty) {
    for (final segment in source.path!.split('.')) {
      if (items is Map<String, dynamic>) {
        items = items[segment];
      } else if (items is List && int.tryParse(segment) != null) {
        items = items[int.parse(segment)];
      } else {
        throw RemoteDataException('Invalid path: ${source.path}');
      }
    }
  }

  if (items is! List) {
    throw RemoteDataException('Response is not an array');
  }

  final labelField = source.label;
  final valueField = source.value ?? source.label;

  final options = items.map<RemoteOption>((item) {
    if (item is Map<String, dynamic>) {
      return RemoteOption(
        label: item[labelField]?.toString() ?? '?',
        value: item[valueField],
      );
    }
    return RemoteOption(label: item.toString(), value: item);
  }).toList();

  // Try to get total count from x-total-count header (common REST convention)
  int? totalCount;
  final totalHeader = response.headers['x-total-count'];
  if (totalHeader != null) {
    totalCount = int.tryParse(totalHeader);
  }

  return FetchResult(options: options, totalCount: totalCount);
}

class RemoteDataException implements Exception {
  final String message;
  RemoteDataException(this.message);
  @override
  String toString() => message;
}
