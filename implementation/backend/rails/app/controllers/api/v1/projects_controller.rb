# frozen_string_literal: true

module Api
  module V1
    class ProjectsController < ApplicationController
      def index
        projects = Project.includes(:organization).order(:name)
        render json: projects.as_json(include: :organization)
      end

      def show
        project = Project.find(params[:id])
        render json: project.as_json(include: :organization)
      end

      def create
        project = Project.new(project_params)
        if project.save
          AuditEvent.create!(action: "create", resource_type: "Project", resource_id: project.id)
          render json: project, status: :created
        else
          render json: { errors: project.errors.full_messages }, status: :unprocessable_entity
        end
      end

      def update
        project = Project.find(params[:id])
        if project.update(project_params)
          render json: project
        else
          render json: { errors: project.errors.full_messages }, status: :unprocessable_entity
        end
      end

      private

      def project_params
        params.require(:project).permit(:organization_id, :name, :slug, :description)
      end
    end
  end
end
