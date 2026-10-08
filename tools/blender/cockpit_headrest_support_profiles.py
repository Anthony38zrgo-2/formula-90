import math

import numpy
from mathutils import Vector


def prepare_support_triangles(objects, conversion):
    triangles = []
    owners = []
    for object_index, scene_object in enumerate(objects):
        scene_object.data.calc_loop_triangles()
        half_count = len(scene_object.data.vertices) // 2
        positions = [conversion @ scene_object.matrix_world @ vertex.co for vertex in scene_object.data.vertices]
        selected = [triangle for triangle in scene_object.data.loop_triangles if all(index < half_count for index in triangle.vertices)]
        triangles.extend([list(positions[index]) for index in triangle.vertices] for triangle in selected)
        owners.extend(object_index for triangle in selected)
    return numpy.asarray(triangles), numpy.asarray(owners)


def support_profile(outer_position, lower_height, upper_height, triangles):
    triangles, owners = triangles
    direction = Vector((outer_position.x, outer_position.y + 0.18, 0)).normalized()
    normal = numpy.asarray((-direction.y, direction.x, 0))
    center = numpy.asarray((0, -0.18, 0))
    distances = (triangles - center) @ normal
    selected_mask = (distances.min(axis=1) <= 0.0000001) & (distances.max(axis=1) >= -0.0000001)
    selected = triangles[selected_mask]
    selected_owners = owners[selected_mask]
    nodes = []
    neighbors = []
    edge_owners = {}

    def node_for(position):
        for index, node in enumerate(nodes):
            if (position - node).length < 0.00002:
                return index
        nodes.append(position)
        neighbors.append(set())
        return len(nodes) - 1

    for triangle, owner in zip(selected, selected_owners):
        points = []
        for first_array, second_array in zip(triangle, numpy.roll(triangle, -1, axis=0)):
            first = Vector(first_array)
            second = Vector(second_array)
            first_distance = (first - Vector(center)).dot(Vector(normal))
            second_distance = (second - Vector(center)).dot(Vector(normal))
            if abs(first_distance) < 0.0000001:
                points.append(first)
            if first_distance * second_distance < 0:
                points.append(first.lerp(second, first_distance / (first_distance - second_distance)))
        unique = []
        for point in points:
            if not any((point - other).length < 0.00002 for other in unique):
                unique.append(point)
        if len(unique) == 2 and all((point - Vector(center)).dot(direction) > 0 for point in unique):
            first_index, second_index = [node_for(point) for point in unique]
            if first_index != second_index:
                neighbors[first_index].add(second_index)
                neighbors[second_index].add(first_index)
                edge_owners.setdefault(tuple(sorted((first_index, second_index))), set()).add(int(owner))
    if not nodes:
        raise RuntimeError('No support intersection profile')
    current_index = min(range(len(nodes)), key=lambda index: (nodes[index] - outer_position).length)
    if (nodes[current_index] - outer_position).length > 0.0001:
        raise RuntimeError('The support profile does not start at the cockpit rim: ' + str(list(outer_position)))
    points = [nodes[current_index]]
    previous_index = None
    current_owner = 0
    for iteration in range(len(nodes) + 2):
        choices = [index for index in neighbors[current_index] if index != previous_index]
        if not choices:
            raise RuntimeError('The cockpit support profile ended above the padding base: ' + str(list(outer_position)) + ' endpoint ' + str(list(nodes[current_index])) + ' nearest ' + str(sorted(((point - nodes[current_index]).length, list(point), len(neighbors[index])) for index, point in enumerate(nodes))[:5]))
        owned_choices = [index for index in choices if current_owner in edge_owners[tuple(sorted((current_index, index)))]]
        if not owned_choices and current_owner == 0:
            current_owner = 1
            owned_choices = [index for index in choices if current_owner in edge_owners[tuple(sorted((current_index, index)))]]
        next_index = min(owned_choices or choices, key=lambda index: nodes[index].z)
        previous_index, current_index = current_index, next_index
        points.append(nodes[current_index])
        if points[-1].z <= lower_height:
            first, second = points[-2:]
            points[-1] = first.lerp(second, (lower_height - first.z) / (second.z - first.z))
            break
    else:
        raise RuntimeError('The cockpit support profile could not reach its base')
    if upper_height >= outer_position.z - 0.000001:
        points[0] = outer_position.copy()
    else:
        for index, (first, second) in enumerate(zip(points, points[1:])):
            if first.z >= upper_height >= second.z:
                upper = first.lerp(second, (upper_height - first.z) / (second.z - first.z))
                points = [upper] + points[index + 1:]
                break
    cleaned = [points[0]]
    for point in points[1:]:
        if (point - cleaned[-1]).length > 0.0000002:
            cleaned.append(point)
    return list(reversed(cleaned))


def cross_section_positions(inner_position, profile, lower_height, upper_height):
    center = Vector((0, -0.18, 0))
    direction = Vector((inner_position.x, inner_position.y + 0.18, 0)).normalized()
    inner_radius = (inner_position - center).length
    lower_radius = (profile[0] - center).dot(direction)
    upper_radius = (profile[-1] - center).dot(direction)
    radius = min(0.004, (lower_radius - inner_radius) * 0.2, (upper_height - lower_height) * 0.2)
    if radius <= 0:
        raise RuntimeError('The padding cavity extends beyond its support')
    positions = []
    parameters = []
    attached = []

    def add(radial_position, height, parameter, attachment=False):
        point = center + direction * radial_position
        point.z = height
        positions.append(point)
        parameters.append(parameter)
        attached.append(attachment)

    for index in range(5):
        angle = math.pi + math.pi * index / 8
        add(inner_radius + radius + radius * math.cos(angle), lower_height + radius + radius * math.sin(angle), index / 4)
    for index in range(1, 4):
        add(inner_radius + radius + (lower_radius - inner_radius - radius) * index / 3, lower_height, 1 + index / 3, index == 3)
    distances = [0.0]
    for first, second in zip(profile, profile[1:]):
        distances.append(distances[-1] + (second - first).length)
    for point, distance in zip(profile[1:], distances[1:]):
        add((point - center).dot(direction), point.z, 2 + distance / distances[-1], True)
    inner_upper = upper_height - 0.002
    outer_upper = profile[-1].z
    for index in range(1, 5):
        fraction = index / 4
        add(upper_radius + (inner_radius + radius - upper_radius) * fraction, outer_upper + (inner_upper - outer_upper) * fraction, 3 + fraction)
    for index in range(1, 5):
        angle = math.pi / 2 + math.pi * index / 8
        add(inner_radius + radius + radius * math.cos(angle), inner_upper - radius + radius * math.sin(angle), 4 + index / 4)
    for index, fraction in enumerate((0.75, 0.5, 0.25)):
        add(inner_radius, lower_height + radius + (inner_upper - lower_height - 2 * radius) * fraction, 5 + (index + 1) / 4)
    return positions, parameters, attached


def closest_support_point(position, contour, triangles):
    center = Vector((0, -0.18, 0))
    direction = Vector((position.x, position.y + 0.18, 0)).normalized()
    normal = Vector((-direction.y, direction.x, 0))
    intersections = []
    for first, second in zip(contour, contour[1:] + contour[:1]):
        first_distance = (first - center).dot(normal)
        second_distance = (second - center).dot(normal)
        if abs(first_distance) < 0.0000001:
            intersections.append(first)
        if first_distance * second_distance < 0:
            intersections.append(first.lerp(second, first_distance / (first_distance - second_distance)))
    intersections = [point for point in intersections if (point - center).dot(direction) > 0]
    outer = min(intersections, key=lambda point: (point - center).length)
    profile = support_profile(outer, 0.17, outer.z, triangles)
    candidates = []
    for first, second in zip(profile, profile[1:]):
        displacement = second - first
        fraction = min(1, max(0, (position - first).dot(displacement) / displacement.length_squared))
        candidates.append(first.lerp(second, fraction))
    return min(candidates, key=lambda point: (point - position).length)
