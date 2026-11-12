# frozen_string_literal: true

puts "Seeding APSInsight..."

org = Organization.find_or_create_by!(slug: "aspf") do |o|
  o.name = "Agriculture Service of Product Farm"
  o.description = "ASPF organization"
end

project = Project.find_or_create_by!(organization: org, slug: "soil-monitoring") do |p|
  p.name = "Soil Monitoring Pilot"
  p.description = "Pilot project for soil sensor and sample data"
end

dataset = Dataset.find_or_create_by!(project: project, slug: "soil-samples-2026") do |d|
  d.organization = org
  d.name = "Soil Samples 2026"
  d.status = "active"
  d.description = "Initial soil sample dataset"
  d.metadata = { region: "Uttar Pradesh", crop: "wheat" }
end

# Roles
admin_role = Role.find_or_create_by!(key: "org_admin") { |r| r.name = "Organization Admin" }
researcher_role = Role.find_or_create_by!(key: "researcher") { |r| r.name = "Researcher" }

# Sample user + membership
user = User.find_or_create_by!(email: "admin@aspf.example") do |u|
  u.name = "ASPF Admin"
end
Membership.find_or_create_by!(user: user, organization: org, role: admin_role)

# Farm / Field / Plot hierarchy
farm = Farm.find_or_create_by!(organization: org, slug: "pilot-farm") do |f|
  f.name = "Pilot Farm"
  f.project = project
  f.area_hectares = 12.5
  f.location = "Gaya, Bihar"
end
field = Field.find_or_create_by!(farm: farm, name: "North Field") do |f|
  f.area_hectares = 5.0
end
plot = Plot.find_or_create_by!(field: field, name: "Plot A1")

puts "Seeded organization: #{org.name}, project: #{project.name}, dataset: #{dataset.name}, farm: #{farm.name}"